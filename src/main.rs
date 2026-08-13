mod infrai_client;
mod queue_worker;

use infrai_client::InfraiClient;
use queue_worker::{dead_letter_and_schedule, decide_failure, FailureDecision, LegalJob};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let follow_up_url = env::var("FOLLOW_UP_URL")?;
    let job = LegalJob {
        matter_id: "MAT-2026-0142".into(),
        document_id: "DOC-88".into(),
        deadline: "2026-08-20".into(),
        attempts: 3,
    };

    match decide_failure(job.attempts, 3) {
        FailureDecision::Retry { next_attempt } => println!("retry delivery at attempt {next_attempt}"),
        FailureDecision::DeadLetter => {
            let job_id = dead_letter_and_schedule(&InfraiClient::from_env()?, &job, &follow_up_url).await?;
            println!("dead-lettered matter {}; follow-up job {job_id}", job.matter_id);
        }
    }
    Ok(())
}

