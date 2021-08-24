mod api_client;
mod binary_search;
mod lol_structs;

use api_client::ApiClient;
use binary_search::{search_match, search_player};
use lol_structs::{Match, MatchHistory, PlayerDto};

use futures::future::join_all;
use serde_json::{self, Value};

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    thread, time,
};

pub async fn find_first_matches(region: &String) -> Result<MatchHistory, reqwest::Error> {
    let client = ApiClient::new(&region);

    let account_id = match region.as_str() {
        "BR1" => "9tve8bFn1Oi5P0YHqhtS2URhZ532dY-4Nkes_eq0683Y",
        "EUN1" => "0tYnLg8tRcpgmwN7kzn1e1S6SLGllVneTLc8Xl1ZXQMjfA",
        "EUW1" => "ckB1ZjtW6D7xRxRopyN3vlp7iMpJVOFtI8PyR14y-3TJAg",
        "LA1" => "93mFcFVrrjb9-DrPeQFCbmefHA0uldeTtKff9VutzLxq3qE",
        "LA2" => "fSRnLCdJuWIt9TxqJE4dIXxk9EIgAHJYYfXoqTeIFoWymiHpdN8MljHs",
        "NA1" => "w9Ay-9buvsBnLA9t6hp2dOfcjCHnjsVcUXAd5Yj_Qthmn2pIj1CsBETF",
        "OC1" => "uQ5xfSUz-D-iVENfVVYu8PfqmCSzmE_lXPDywACYyBYQkdOVaeY5Ph8F",
        "RU1" => "09aPcxfByeNGN4O_Od6ePa8OUH0900GP4-sWqe1no9vHzt29vi-D9MxE",
        "TR1" => "i7Dkz-sGeRQcFDDq4wIfFMOzea7w7XsqLM4k3HD4MeZmH7ggD8Y9BWaD",
        "JP1" => "D_ETtoMlfJLjVzQdABXb--GvH8agwYslSe29kC0OnCS6fa5aM0fn52FR",
        "KR" => "2wVcuPh3YDciNfzwxsaPIy79g-Iqnz9dTBCGmm9Z5EjB",
        _ => "",
    };

    let result = client.fetch_player(account_id.to_owned()).await?;

    let deserealized_result: Result<MatchHistory, serde_json::Error> =
        serde_json::from_str(&result);

    Ok(deserealized_result.unwrap())
}

pub async fn mine(region: String) {
    let new_matches = Arc::new(Mutex::new(Vec::with_capacity(1000) as Vec<Match>));
    let new_players = Arc::new(Mutex::new(Vec::with_capacity(10000) as Vec<PlayerDto>));
    let matches_to_find = Arc::new(Mutex::new(VecDeque::new()));
    let players_to_find = Arc::new(Mutex::new(VecDeque::new()));
    let region_clone = region.clone();

    let first_matches = find_first_matches(&region).await.unwrap();

    {
        let mut matches_queu = matches_to_find.lock().unwrap();

        for game in first_matches.matches {
            if game.queue == 420 {
                (*matches_queu).push_back(game.gameId);
            }
        }
    }

    //Match minner
    // Atomic references for match minner
    let mm_matches_to_find = Arc::clone(&matches_to_find);
    let mm_players_to_find = Arc::clone(&players_to_find);
    let mm_new_players = Arc::clone(&new_players);
    let mm_new_matches = Arc::clone(&new_matches);

    let match_minner = tokio::spawn(async move {
        let client = ApiClient::new(&region);

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
                    api_calls.push(client.fetch_match(id));
                }
                let resolved_api_calls = join_all(api_calls).await;

                let time_used_to_find = time::Instant::now();
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

                                        match search_match(&*new_matches, &match_game.gameId) {
                                            Ok(_) => {}
                                            Err(index) => {
                                                (*new_matches).insert(index, match_game);
                                            }
                                        }
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

                let time_to_sleep = time::Duration::new(24, 0) - time_used_to_find.elapsed();
                thread::sleep(time_to_sleep);
            }
        }
    });

    //PlayerIds minner
    // Atomic references for player minner
    let pm_players_to_find = Arc::clone(&players_to_find);
    let pm_matches_to_find = Arc::clone(&matches_to_find);
    let pm_new_matches = new_matches.clone();

    let player_minner = tokio::spawn(async move {
        let client = ApiClient::new(&region_clone);

        loop {
            let mut api_calls = Vec::new();
            let mut player_ids: Vec<String> = Vec::new();
            let num_of_request: usize = 10;
            {
                let mut players_to_find = pm_players_to_find.lock().unwrap();

                if (*players_to_find).len() > 10 {
                    for _ in 0..num_of_request {
                        player_ids.push((*players_to_find).pop_front().unwrap());
                    }
                }
            }

            if player_ids.len() > 0 {
                for id in player_ids {
                    api_calls.push(client.fetch_player(id));
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
                let time_to_sleep = time::Duration::new(24, 0) - time_used_to_find.elapsed();
                thread::sleep(time_to_sleep);
            }
        }
    });

    futures::join!(match_minner, player_minner);
}
