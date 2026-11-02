use super::*;

impl Client {
    pub fn search(
        &self,
        search: &super::super::Search,
    ) -> Result<Receipt<super::super::Matches>, Error> {
        let call = search.call()?;
        let identity = self.binding()?;
        let bearer = Zeroizing::new(format!("Bearer {}", self.token()?));
        let body = Zeroizing::new(
            serde_json::to_vec(&call.body).map_err(|_| super::super::fault::input())?,
        );
        let url = format!("https://open.feishu.cn/open-apis/{}", call.path);
        let record = wire::invoke(
            &self.agent,
            http::Request {
                method: call.method,
                url: &url,
                headers: &[
                    ("Authorization", &bearer),
                    ("Content-Type", "application/json"),
                ],
                body: Some(&body),
            },
        )?;
        let data = search.decode(&record.body.0).map_err(|_| {
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
