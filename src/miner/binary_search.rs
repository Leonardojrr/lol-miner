use super::lol_structs::{Match, PlayerDto};

pub fn search_match(matches: &Vec<Match>, id: &u64) -> Result<usize, usize> {
    if matches.len() == 0 {
        return Err(0);
    }

    let mut left: usize = 0;
    let mut right: usize = matches.len() - 1;
    let mut past_middle: usize = 0;

    loop {
        let middle: usize = (left + right) / 2;
        let item = &matches[middle].gameId;

        if past_middle == middle {
            if id > item {
                return Err(middle + 1);
            }

            if id < item {
                return Err(middle);
            }
        }

        if id == item {
            return Ok(middle);
        }

        if id > item {
            left = middle + 1;
        }

        if id < item {
            if middle == 0 {
                right = 0
            } else {
                right = middle - 1;
            }
        }

        past_middle = middle;
    }
}

pub fn search_player(players: &Vec<PlayerDto>, id: &String) -> Result<usize, usize> {
    if players.len() == 0 {
        return Err(0);
    }

    let mut left: usize = 0;
    let mut right: usize = players.len() - 1;
    let mut past_middle: usize = 0;

    loop {
        let middle: usize = (left + right) / 2;
        let item = &players[middle].currentAccountId;

        if past_middle == middle {
            if id > item {
                return Err(middle + 1);
            }

            if id < item {
                return Err(middle);
            }
        }

        if id == item {
            return Ok(middle);
        }

        if id > item {
            left = middle + 1;
        }

        if id < item {
            if middle == 0 {
                right = 0;
            } else {
                right = middle - 1;
            }
        }

        past_middle = middle;
    }
}
