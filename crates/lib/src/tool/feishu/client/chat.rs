use super::*;
use crate::feishu::chat;

impl Client {
    pub fn chat(&self, get: &chat::Get) -> Result<Receipt<chat::Chat>, Error> {
        let call = get.call()?;
        let identity = self.binding()?;
        let bearer = Zeroizing::new(format!("Bearer {}", self.token()?));
        let url = format!("https://open.feishu.cn/open-apis/{}", call.path);
        let record = wire::invoke(
            &self.agent,
            http::Request {
                method: call.method,
                url: &url,
                headers: &[("Authorization", &bearer)],
                body: None,
            },
        )?;
        let data = get.decode(&record.body.0).map_err(|_| {
            let mut error = response();
            error.request = record.request.clone();
            error
        })?;
        Ok(Receipt {
            identity,
            action: call.action.into(),
            observed: now(),
            request: record.request,
            outcome: Outcome::Observed,
            data,
        })
    }
}
