use reqwest::Client;

pub struct ApiClient {
    client: Client,
    hostname: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(region: &str) -> ApiClient {
        ApiClient {
            client: Client::new(),
            hostname: format!("https://{}.api.riotgames.com", region),
            api_key: String::from(""), // <--------- Put your Api key here
        }
    }

    pub async fn fetchMatch(&self, match_id: u64) -> Result<String, reqwest::Error> {
        let url = format!(
            "{}/lol/match/v4/matches/{}?api_key={}",
            self.hostname, match_id, self.api_key
        );

        Ok(self.client.get(&url).send().await?.text().await?)
    }

    pub async fn fetchPlayer(&self, account_id: String) -> Result<String, reqwest::Error> {
        let url = format!(
            "{0}/lol/match/v4/matchlists/by-account/{1}?api_key={2}",
            self.hostname, account_id, self.api_key
        );

        Ok(self.client.get(&url).send().await?.text().await?)
    }
}
