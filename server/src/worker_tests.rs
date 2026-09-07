use super::*;
use std::io::Cursor;

fn command(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]);
    command
}

#[test]
fn rejects_input_when_source_exceeds_limit() {
    let source = "x".repeat(wire::INPUT_LIMIT + 1);
    let result = Worker::spawn(command("exit 0"), source, Budget::default());
    assert!(matches!(result, Err(Failure::InputLimit)));
}

#[test]
fn rejects_frame_when_advertised_output_exceeds_limit() {
    let mut input = Cursor::new(u32::MAX.to_be_bytes());
    let result = wire::read::<Outcome>(&mut input, wire::OUTPUT_LIMIT);
    assert!(result.is_err());
}

#[test]
fn separates_computation_deadline_when_worker_becomes_ready() {
    let mut worker = Worker::spawn(
        command(r#"printf '\000\000\000\007"Ready"'; exec sleep 60"#),
        "let x = 1".into(),
        Budget::default(),
    )
    .unwrap();
    let event = worker.events.recv_timeout(Duration::from_secs(5)).unwrap();
    let Event::Ready(at) = event else {
        panic!("worker did not become ready")
    };
    worker.ready = true;
    worker.deadline = at;
    let result = worker.poll();
    assert!(matches!(result, Some(Err(Failure::ComputationTimeout))));
}

#[test]
fn reports_startup_timeout_when_worker_never_announces_readiness() {
    let mut worker =
        Worker::spawn(command("exec sleep 60"), String::new(), Budget::default()).unwrap();
    worker.deadline = Instant::now();
    let result = worker.poll();
    assert!(matches!(result, Some(Err(Failure::StartupTimeout))));
}

#[test]
fn reports_transport_failure_when_worker_crashes() {
    let worker = Worker::spawn(command("exit 17"), String::new(), Budget::default()).unwrap();
    let event = worker.events.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(event, Event::Finished(Err(Failure::Transport), _)));
}

#[test]
fn reaps_child_and_joins_io_when_worker_is_dropped() {
    let worker = Worker::spawn(command("exec sleep 60"), String::new(), Budget::default()).unwrap();
    let pid = worker.child.id();
    drop(worker);
    let exists = Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(!exists);
}

#[test]
fn reads_valid_outcome_when_transport_round_trips() {
    let mut bytes = Vec::new();
    wire::write(
        &mut bytes,
        &Outcome::Changed("let x = 1\n".into()),
        wire::OUTPUT_LIMIT,
    )
    .unwrap();
    let outcome = wire::read::<Outcome>(&mut Cursor::new(bytes), wire::OUTPUT_LIMIT).unwrap();
    assert!(matches!(outcome, Outcome::Changed(text) if text == "let x = 1\n"));
}

#[test]
fn starts_computation_budget_when_ready_event_arrives_before_startup_deadline() {
    let mut worker =
        Worker::spawn(command("exec sleep 60"), String::new(), Budget::default()).unwrap();
    let (sender, events) = mpsc::sync_channel(2);
    worker.events = events;
    let ready = Instant::now();
    sender.send(Event::Ready(ready)).unwrap();

    let result = worker.poll();

    assert!(result.is_none());
    assert_eq!(worker.deadline, ready + Duration::from_secs(2));
}

#[test]
fn rejects_completed_output_when_computation_deadline_was_exceeded() {
    let mut worker =
        Worker::spawn(command("exec sleep 60"), String::new(), Budget::default()).unwrap();
    let (sender, events) = mpsc::sync_channel(2);
    worker.events = events;
    let ready = Instant::now();
    sender.send(Event::Ready(ready)).unwrap();
    sender
        .send(Event::Finished(
            Ok(Outcome::Unchanged),
            ready + Duration::from_secs(3),
        ))
        .unwrap();

    let result = worker.poll();

    assert!(matches!(result, Some(Err(Failure::ComputationTimeout))));
}
