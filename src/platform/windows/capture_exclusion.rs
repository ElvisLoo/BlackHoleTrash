pub const REQUIRED_AFFINITY: u32 = 0x11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AffinityObservation {
    SetFailed(u32),
    ReadFailed(u32),
    Reported(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureExclusionError {
    stage: &'static str,
    observation: AffinityObservation,
}

fn validate_affinity(
    stage: &'static str,
    observation: AffinityObservation,
) -> Result<(), CaptureExclusionError> {
    match observation {
        AffinityObservation::Reported(REQUIRED_AFFINITY) => Ok(()),
        observation => Err(CaptureExclusionError { stage, observation }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exact_exclude_from_capture_value() {
        assert_eq!(
            validate_affinity("initial setup", AffinityObservation::Reported(0x11)),
            Ok(())
        );
    }

    #[test]
    fn rejects_set_failure_with_error_code() {
        let error = validate_affinity("initial setup", AffinityObservation::SetFailed(87))
            .unwrap_err();
        assert_eq!(error.stage, "initial setup");
        assert_eq!(error.observation, AffinityObservation::SetFailed(87));
    }

    #[test]
    fn rejects_read_failure_with_error_code() {
        let error = validate_affinity(
            "post-show verification",
            AffinityObservation::ReadFailed(5),
        )
        .unwrap_err();
        assert_eq!(error.observation, AffinityObservation::ReadFailed(5));
    }

    #[test]
    fn rejects_mismatched_reported_value() {
        let error = validate_affinity("monitor rebuild", AffinityObservation::Reported(0x01))
            .unwrap_err();
        assert_eq!(error.observation, AffinityObservation::Reported(0x01));
    }
}
