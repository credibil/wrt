use credibil_error::Error;
use tracing::{error, warn};

pub fn log_with_metrics(err: &Error, service: &str, topic: &str) {
    match err {
        Error::ServiceUnavailable(description) => {
            error!(
                monotonic_counter.processing_errors = 1,
                service = %service,
                topic = %topic,
                description
            );
        }
        Error::BadGateway(description) => {
            error!(
                monotonic_counter.external_errors = 1,
                service = %service,
                topic = %topic,
                description
            );
        }
        Error::ServerError(description) => {
            error!(
                monotonic_counter.runtime_errors = 1,
                service = %service,
                description
            );
        }
        Error::BadRequest(description) => {
            warn!(
                monotonic_counter.parsing_errors = 1,
                service = %service,
                topic = %topic,
                description
            );
        }
        Error::Unauthorized(description) => {
            warn!(
                monotonic_counter.authorization_errors = 1,
                service = %service,
                description
            );
        }
        Error::NotFound(description) => {
            warn!(
                monotonic_counter.not_found_errors = 1,
                service = %service,
                description
            );
        }
        Error::Gone(description) => {
            warn!(
                monotonic_counter.stale_data = 1,
                service = %service,
                topic = %topic,
                description
            );
        }
        Error::ImATeaPot(description) => {
            warn!(
                monotonic_counter.other_errors = 1,
                service = %service,
                description
            );
        }
    }
}
