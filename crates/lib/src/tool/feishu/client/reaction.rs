use super::*;
use crate::feishu::reaction;

impl Client {
    pub fn reaction(&self, create: &reaction::Create) -> Result<Receipt<reaction::Created>, Error> {
        let call = create.call()?;
        let identity = self.binding()?;
        let record = self.react(&call)?;
        let data = create
            .decode(&record.body.0, &identity)
            .map_err(|_| tracked(&record))?;
        Ok(Receipt {
            identity,
            action: call.action.into(),
            observed: now(),
            request: record.request,
            outcome: Outcome::Accepted,
            data,
        })
    }

    pub fn reactions(&self, page: &reaction::Page) -> Result<Receipt<reaction::Reactions>, Error> {
        let call = page.call()?;
        let identity = self.binding()?;
        let record = self.react(&call)?;
        let data = page.decode(&record.body.0).map_err(|_| tracked(&record))?;
        Ok(Receipt {
            identity,
            action: call.action.into(),
            observed: now(),
            request: record.request,
            outcome: Outcome::Observed,
            data,
        })
    }

    fn react(&self, call: &super::super::message::Call) -> Result<wire::Record, Error> {
        let bearer = Zeroizing::new(format!("Bearer {}", self.token()?));
        let body = call
            .body
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|_| super::super::fault::input())?
            .map(Zeroizing::new);
        let url = format!("https://open.feishu.cn/open-apis/{}", call.path);
        wire::invoke(
            &self.agent,
            http::Request {
                method: call.method,
                url: &url,
                headers: &[
                    ("Authorization", &bearer),
                    ("Content-Type", "application/json"),
                ],
                body: body.as_deref().map(Vec::as_slice),
            },
        )
    }
}

fn tracked(record: &wire::Record) -> Error {
    let mut error = response();
    error.request = record.request.clone();
    error
}
