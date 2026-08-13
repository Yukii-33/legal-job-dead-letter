use crate::infrai_client::{InfraiClient, InfraiError};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct LegalJob {
    pub matter_id: String,
    pub document_id: String,
    pub deadline: String,
    pub attempts: u8,
}

#[derive(Debug, PartialEq)]
pub enum FailureDecision {
    Retry { next_attempt: u8 },
    DeadLetter,
}

pub fn decide_failure(attempts: u8, max_attempts: u8) -> FailureDecision {
    if attempts >= max_attempts {
        FailureDecision::DeadLetter
    } else {
        FailureDecision::Retry { next_attempt: attempts + 1 }
    }
}

#[derive(Serialize)]
struct DeadLetterPayload<'a> {
    matter_id: &'a str,
    document_id: &'a str,
    deadline: &'a str,
    failed_stage: &'static str,
}

pub async fn dead_letter_and_schedule(
    client: &InfraiClient,
    job: &LegalJob,
    follow_up_url: &str,
) -> Result<String, InfraiError> {
    let handoff_key = format!("matter-{}-document-{}", job.matter_id, job.document_id);
    client.publish(
        &DeadLetterPayload {
            matter_id: &job.matter_id,
            document_id: &job.document_id,
            deadline: &job.deadline,
            failed_stage: "signed_document_delivery",
        },
        &format!("{handoff_key}-dead-letter"),
    ).await?;

    client.create_follow_up(
        "0 9 * * *",
        follow_up_url,
        &format!("{handoff_key}-deadline-follow-up"),
    ).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poison_delivery_moves_to_dead_letter_at_attempt_limit() {
        assert_eq!(decide_failure(3, 3), FailureDecision::DeadLetter);
        assert_eq!(decide_failure(2, 3), FailureDecision::Retry { next_attempt: 3 });
    }
}

