use lsp_server::Message;
use serde_json::{Value, json};
use std::cell::Cell;
use std::io::{BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::Duration;
use wait_timeout::ChildExt;

pub struct Client {
    child: Child,
    input: ChildStdin,
    messages: Receiver<Message>,
    started: Cell<bool>,
    reader: Option<JoinHandle<()>>,
}

impl Client {
    pub fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_iris-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (sender, messages) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            while let Some(message) = Message::read(&mut reader).unwrap() {
                if sender.send(message).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            messages,
            started: Cell::new(false),
            reader: Some(reader),
        }
    }

    pub fn send(&mut self, message: &Value) {
        let body = serde_json::to_vec(message).unwrap();
        write!(self.input, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        self.input.write_all(&body).unwrap();
        self.input.flush().unwrap();
    }

    pub fn receive(&self) -> Value {
        // macOS test runs observed 15-second startup but prompt later responses.
        let seconds = if self.started.get() { 5 } else { 45 };
        let message = self
            .messages
            .recv_timeout(Duration::from_secs(seconds))
            .unwrap();
        self.started.set(true);
        serde_json::to_value(message).unwrap()
    }

    pub fn response(&self) -> Value {
        loop {
            let message = self.messages.recv_timeout(Duration::from_secs(50)).unwrap();
            match message {
                Message::Response(response) => return serde_json::to_value(response).unwrap(),
                Message::Notification(notification) => {
                    assert_eq!(notification.method, "window/logMessage");
                }
                Message::Request(_) => panic!("unexpected server request"),
            }
        }
    }

    pub fn initialize(&mut self) -> Value {
        self.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"capabilities":{}}}));
        let result = self.receive();
        self.send(&json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
        result
    }

    pub fn open(&mut self, uri: &str, text: &str) -> Value {
        self.send(&json!({"jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"languageId":"iris","version":1,"text":text}}}));
        self.receive()
    }

    pub fn exit(&mut self, expected: i32) {
        self.send(&json!({"jsonrpc":"2.0","method":"exit"}));
        let status = self
            .child
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .expect("server must exit with stdin still open");
        assert_eq!(status.code(), Some(expected));
    }

    pub fn shutdown(&mut self) {
        self.send(&json!({"jsonrpc":"2.0","id":99,"method":"shutdown","params":null}));
        assert_eq!(self.receive(), json!({"id":99,"result":null}));
        self.exit(0);
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            self.child.kill().unwrap();
            self.child.wait().unwrap();
        }
        if let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
    }
}
