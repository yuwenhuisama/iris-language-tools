import { spawn } from "node:child_process";
import type { ChildProcessWithoutNullStreams } from "node:child_process";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import type { LanguageClientOptions, MessageTransports } from "vscode-languageclient/node";

export class StartupInterrupted extends Error {
  constructor(readonly timedOut = false) {
    super(timedOut ? "Language server initialization timed out after 45000ms" : "Language server startup interrupted");
  }
}

export class ServerClient extends LanguageClient {
  private retired = false;
  private starting = true;
  private retirement: Promise<void> | undefined;
  private interrupt: ((error: StartupInterrupted) => void) | undefined;
  private readonly cancellation = new Promise<never>((_, reject) => { this.interrupt = reject; });
  private readonly processOwner: { child?: ChildProcessWithoutNullStreams };

  constructor(command: string, options: LanguageClientOptions) {
    const owner: { child?: ChildProcessWithoutNullStreams } = {};
    super("iris", "Iris Language Server", async () => {
      if (this.retired) throw new StartupInterrupted();
      const child = spawn(command, [], { cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath });
      owner.child = child;
      await new Promise<void>((resolve, reject) => {
        child.once("spawn", resolve);
        child.once("error", reject);
      });
      if (this.retired) throw new StartupInterrupted();
      return child;
    }, options);
    this.processOwner = owner;
  }

  override async start(): Promise<void> {
    const timer = setTimeout(() => this.cancelStartup(true), 45000);
    try {
      const startup = super.start();
      const readiness = super.start();
      await Promise.race([startup, readiness, this.cancellation]);
    } finally {
      this.starting = false;
      clearTimeout(timer);
    }
  }

  cancelStartup(timedOut = false): void {
    if (!this.starting || this.retired) return;
    this.retired = true;
    this.interrupt?.(new StartupInterrupted(timedOut));
  }

  protected override async createMessageTransports(encoding: string): Promise<MessageTransports> {
    const transport = await super.createMessageTransports(encoding);
    if (this.retired) throw new StartupInterrupted();
    const writer = transport.writer;
    return { reader: transport.reader, writer: {
      onError: writer.onError, onClose: writer.onClose,
      end: () => writer.end(), dispose: () => writer.dispose(),
      write: async message => {
        if (this.retired) throw new StartupInterrupted();
        await writer.write(message);
        if (this.retired) throw new StartupInterrupted();
      },
    } };
  }

  override stop(timeout?: number): Promise<void> {
    return !this.retired && this.isRunning() ? super.stop(timeout) : Promise.resolve();
  }

  override dispose(): Promise<void> {
    this.retirement ??= this.retire();
    return this.retirement;
  }

  private async retire(): Promise<void> {
    this.cancelStartup();
    try {
      await super.dispose();
    } finally {
      this.retired = true;
      const child = this.processOwner.child;
      const exited = child?.pid !== undefined && child.exitCode === null && child.signalCode === null
        ? new Promise<void>(resolve => child.once("exit", () => resolve())) : Promise.resolve();
      child?.kill("SIGKILL");
      child?.stdin.destroy();
      child?.stdout.destroy();
      child?.stderr.destroy();
      await super.handleConnectionClosed();
      await exited;
    }
  }
}
