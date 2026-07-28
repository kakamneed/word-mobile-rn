use std::io::{self, Read};

use serde_json::{json, Value};
use word_domain_protocol::v1::execute_v1;

#[derive(Debug, thiserror::Error)]
enum RunnerError {
    #[error("missing command")]
    MissingCommand,
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("failed to read stdin: {0}")]
    ReadStdin(#[from] io::Error),
    #[error("invalid json input: {0}")]
    Json(#[from] serde_json::Error),
    #[error("domain protocol rejected the legacy request: {0}")]
    Protocol(String),
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), RunnerError> {
    let command = std::env::args().nth(1).ok_or(RunnerError::MissingCommand)?;
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    println!("{}", execute_command(&command, &input)?);
    Ok(())
}

fn execute_command(command: &str, input: &str) -> Result<String, RunnerError> {
    if command == "execute-v1" {
        return Ok(execute_v1(input));
    }

    let protocol_command = match command {
        "build-session" => "buildSession",
        "evaluate-answer" => "evaluateAnswer",
        "complete-session" => "completeSession",
        _ => return Err(RunnerError::UnknownCommand(command.to_string())),
    };
    let payload: Value = serde_json::from_str(input)?;
    let now = chrono::Utc::now();
    let now_utc = now.to_rfc3339();
    let session_id = format!("sess_{}", now.format("%Y%m%d%H%M%S%f"));
    let request = json!({
        "protocolVersion": 1,
        "command": protocol_command,
        "requestId": format!("legacy-{command}"),
        "context": {
            "nowUtc": now_utc,
            "localDay": now.format("%Y-%m-%d").to_string(),
            "sessionId": session_id,
            "orderingSeed": format!("legacy-{command}")
        },
        "payload": payload
    });
    let response: Value = serde_json::from_str(&execute_v1(&request.to_string()))?;
    if let Some(error) = response.get("error") {
        return Err(RunnerError::Protocol(error.to_string()));
    }
    serde_json::to_string(
        response
            .get("result")
            .ok_or_else(|| RunnerError::Protocol("missing result".to_string()))?,
    )
    .map_err(RunnerError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_v1_command_is_the_shared_dispatcher_without_conversion() {
        let request =
            include_str!("../../../fixtures/domain/v1/requests/study-progress-summary.json");
        let native: Value = serde_json::from_str(&execute_command("execute-v1", request).unwrap())
            .expect("native envelope");
        let shared: Value = serde_json::from_str(&execute_v1(request)).expect("shared envelope");
        assert_eq!(native, shared);
    }

    #[test]
    fn legacy_evaluate_answer_is_only_an_input_adapter() {
        let question = json!({
            "questionId": "q1",
            "questionType": "enToCnInput",
            "entrySourceId": "entry-alpha",
            "word": "alpha",
            "partOfSpeech": "n.",
            "phoneticUs": null,
            "phoneticUk": null,
            "prompt": "alpha",
            "acceptedMeanings": ["alpha meaning"],
            "exampleSentence": null,
            "exampleTranslation": null,
            "choices": null,
            "correctChoiceLabel": null,
            "questionIndex": 0,
            "totalQuestions": 1
        });
        let output: Value = serde_json::from_str(
            &execute_command(
                "evaluate-answer",
                &json!({
                    "question": question,
                    "answer": {
                        "questionId": "q1",
                        "response": "alpha meaning",
                        "responseTimeMs": 100
                    },
                    "answeredAt": "2026-07-28T00:00:00Z"
                })
                .to_string(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(output["outcome"], "correct");
        assert_eq!(output["answeredAt"], "2026-07-28T00:00:00Z");
    }
}
