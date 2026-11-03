use super::*;
use crate::feishu::chat;

impl Client {
    pub fn chat(&self, get: &chat::Get) -> Result<Receipt<chat::Chat>, Error> {
        let call = get.call()?;
        self.observed(&call, |body| get.decode(body))
    }
}
