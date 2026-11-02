use mytools_core::capability::CapabilityError;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct CapabilityExecutor {
    slots: Arc<Semaphore>,
}

impl Default for CapabilityExecutor {
    fn default() -> Self {
        Self::new(8)
    }
}

impl CapabilityExecutor {
    pub fn new(concurrency: usize) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(concurrency)),
        }
    }

    pub async fn execute<F>(&self, capability: &str, handler: F) -> Result<Value, CapabilityError>
    where
        F: FnOnce() -> Result<Value, CapabilityError> + Send + 'static,
    {
        let permit = self.slots.clone().try_acquire_owned().map_err(|_| {
            CapabilityError::handler_failed(
                capability,
                "capability execution limit reached; retry later",
            )
        })?;
        tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            handler()
        })
        .await
        .map_err(|e| CapabilityError::handler_failed(capability, e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_saturation_without_executing_handler() {
        let executor = CapabilityExecutor::new(1);
        let permit = executor
            .slots
            .clone()
            .acquire_owned()
            .await
            .expect("permit");
        let result = executor
            .execute("test.call", || panic!("must not execute"))
            .await;
        assert!(matches!(result, Err(CapabilityError::HandlerFailed { .. })));
        drop(permit);
        assert_eq!(
            executor
                .execute("test.call", || Ok(Value::Bool(true)))
                .await
                .expect("call"),
            Value::Bool(true)
        );
    }

    #[tokio::test]
    async fn releases_permit_after_handler_failure() {
        let executor = CapabilityExecutor::new(1);
        assert!(executor
            .execute("test.call", || Err(CapabilityError::invalid_args("bad")))
            .await
            .is_err());
        assert_eq!(
            executor
                .execute("test.call", || Ok(Value::Null))
                .await
                .expect("call"),
            Value::Null
        );
    }
}
