mod lolStructs;

use futures::future::join_all;
use lolStructs::{Match, MatchHistory, PlayerDto};
use reqwest::Client;
use serde_json::{self, Value};

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread, time,
};

struct ApiClient {
    client: Client,
    hostname: String,
    api_key: String,
}

impl ApiClient {
    fn new(region: &str) -> ApiClient {
        ApiClient {
            client: Client::new(),
            hostname: format!("https://{}.api.riotgames.com", region),
            api_key: String::from("------API KEY------"), // <--------- Put your Api key here
        }
    }

    pub async fn fetchMatch(&self, matchId: u64) -> Result<String, reqwest::Error> {
        let url = format!(
            "{}/lol/match/v4/matches/{}?api_key={}",
            self.hostname, matchId, self.api_key
        );

        Ok(self.client.get(&url).send().await?.text().await?)
    }

    pub async fn fetchPlayer(&self, accountId: String) -> Result<String, reqwest::Error> {
        let url = format!(
            "{}/lol/match/v4/matchlists/by-account/{}?api_key={}",
            self.hostname, accountId, self.api_key
        );

        Ok(self.client.get(&url).send().await?.text().await?)
    }
}

fn search_match(matches: &Vec<Match>, id: &u64) -> Result<usize, usize> {
    let mut left: usize = 0;
    let mut right: usize = matches.len();

    loop {
        let middle: usize = (left + right) / 2;

        if id == &matches[middle].gameId {
            return Ok(middle);
        }

        if left == right {
            if id > &matches[middle].gameId {
                return Err(middle + 1);
            } else {
                return Err(middle - 1);
            }
        }

        if id > &matches[middle].gameId {
            left = middle + 1;
        }

        if id < &matches[middle].gameId {
            right = middle - 1;
        }
    }
}

fn search_player(players: &Vec<PlayerDto>, id: &String) -> Result<usize, usize> {
    let mut left: usize = 0;
    let mut right: usize = players.len();

    loop {
        let middle: usize = (left + right) / 2;

        if id == &players[middle].currentAccountId {
            return Ok(middle);
        }

        if left == right {
            if id > &players[middle].currentAccountId {
                return Err(middle + 1);
            } else {
                return Err(middle - 1);
            }
        }

        if id > &players[middle].currentAccountId {
            left = middle + 1;
        }

        if id < &players[middle].currentAccountId {
            right = middle - 1;
        }
    }
}

async fn mine(region: &'static str) {
    let new_matches = Arc::new(Mutex::new(Vec::with_capacity(1000) as Vec<Match>));
    let new_players = Arc::new(Mutex::new(Vec::with_capacity(10000) as Vec<PlayerDto>));
    let matches_to_find = Arc::new(Mutex::new(VecDeque::new()));
    let players_to_find = Arc::new(Mutex::new(VecDeque::new()));

    //Match minner
    // Atomic references for match minner
    let mm_matches_to_find = Arc::clone(&matches_to_find);
    let mm_players_to_find = Arc::clone(&players_to_find);
    let mm_new_players = Arc::clone(&new_players);
    let mm_new_matches = Arc::clone(&new_matches);

    let match_minner = tokio::spawn(async move {
        let client = ApiClient::new(region);

        loop {
            let mut api_calls = Vec::new();
            let mut match_ids: Vec<u64> = Vec::new();
            let num_of_request: u64 = 10;

            {
                let mut matches_to_find = mm_matches_to_find.lock().unwrap();

                for _ in 0..num_of_request {
                    match_ids.push((*matches_to_find).pop_front().unwrap());
                }
            }

            if match_ids.len() > 0 {
                for id in match_ids {
                    api_calls.push(client.fetchMatch(id));
                }
                let resolved_api_calls = join_all(api_calls).await;
                {
                    let mut new_players = mm_new_players.lock().unwrap();
                    let mut new_matches = mm_new_matches.lock().unwrap();
                    let mut players_to_find = mm_players_to_find.lock().unwrap();

                    for result in resolved_api_calls {
                        match result {
                            Ok(strg) => {
                                let deserialized_match: Result<Match, serde_json::Error> =
                                    serde_json::from_str(&strg);

                                match deserialized_match {
                                    Ok(match_game) => {
                                        for indentity in &match_game.participantIdentities {
                                            let player_id =
                                                indentity.player.currentAccountId.clone();

                                            match search_player(&*new_players, &player_id) {
                                                Ok(_) => {}

                                                Err(index) => {
                                                    (*new_players)
                                                        .insert(index, indentity.player.clone());

                                                    (*players_to_find).push_back(player_id);
                                                }
                                            };
                                        }
                                        (*new_matches).push(match_game);
                                    }
                                    Err(err) => {
                                        let json: Value = serde_json::from_str(&strg).unwrap();
                                        println!("hay un error en esta partida {}", json["gameId"]);
                                        println!("Err {}", err);
                                    }
                                }
                            }
                            Err(_) => (),
                        }
                    }
                }
            }
            thread::sleep(time::Duration::new(24, 0));
        }
    });

    //PlayerIds minner
    // Atomic references for player minner
    let pm_players_to_find = Arc::clone(&players_to_find);
    let pm_matches_to_find = Arc::clone(&matches_to_find);
    let pm_new_matches = new_matches.clone();

    let player_minner = tokio::spawn(async move {
        let client = ApiClient::new(region);
        loop {
            let mut api_calls = Vec::new();
            let mut player_ids: Vec<String> = Vec::new();
            let num_of_request: usize = 10;
            {
                let mut players_to_find = pm_players_to_find.lock().unwrap();

                for _ in 0..num_of_request {
                    player_ids.push((*players_to_find).pop_front().unwrap());
                }
            }

            if player_ids.len() > 0 {
                for id in player_ids {
                    api_calls.push(client.fetchPlayer(id));
                }
                let resolved_api_calls = join_all(api_calls).await;

                let time_used_to_find = time::Instant::now();
                {
                    let mut matches_to_find = pm_matches_to_find.lock().unwrap();
                    for result in resolved_api_calls {
                        match result {
                            Ok(strg) => {
                                let deserialized_matches_history: Result<
                                    MatchHistory,
                                    serde_json::Error,
                                > = serde_json::from_str(&strg);

                                match deserialized_matches_history {
                                    Ok(matches_history) => {
                                        for match_reference in matches_history.matches {
                                            if match_reference.queue == 420 {
                                                {
                                                    let all_matches =
                                                        pm_new_matches.lock().unwrap();

                                                    let game_id = match_reference.gameId.clone();

                                                    match search_match(&*all_matches, &game_id) {
                                                        Ok(_) => {}
                                                        Err(_) => {
                                                            (*matches_to_find)
                                                                .push_back(match_reference.gameId);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                            Err(_) => (),
                        }
                    }
                }
            }
            thread::sleep(time::Duration::new(24, 0));
        }
    });

    futures::join!(match_minner, player_minner);
}

#[tokio::main]
async fn main() {
    // let na_minner_handler = mine("na1");
    // let euw_minner_handler = mine("euw1");
    // let la_minner_handler = mine("la1");

    // futures::join!(na_minner_handler, euw_minner_handler, la_minner_handler);
}
