mod common;
mod neighbors;
mod requests;

use csv::ReaderBuilder;
use std::env;
use std::error::Error;
use std::fs::{read_to_string, File};
use std::path::Path;
use std::time::Instant;
use chrono::{DateTime, Datelike, FixedOffset, Utc};

use common::{Pos, Word, Word1D, COLS, ROWS};
use neighbors::get_neighbors;
use requests::fetch_board_for_date;

use crate::common::{get_index, print_board};

fn read_csv<P: AsRef<Path>>(filename: P) -> Result<Vec<String>, Box<dyn Error>> {
    let file = File::open(filename)?;
    let mut rdr = ReaderBuilder::new().delimiter(b'\t').from_reader(file);
    let mut results: Vec<String> = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let word_res = record.get(0);
        match word_res {
            Some(x) => {
                if x.chars().count() > 2 {
                    results.push(x.to_string())
                }
            }
            None => (),
        }
    }
    Ok(results)
}

fn build_board(data: Vec<char>) -> Vec<Vec<char>> {
    let mut res: Vec<Vec<char>> = vec![vec!['X'; COLS]; ROWS];
    for j in 0..ROWS {
        for i in 0..COLS {
            res[j][i] = data[j * COLS + i].to_uppercase().next().unwrap();
        }
    }
    return res;
}

fn inner(
    board: &Vec<Vec<char>>,
    visited: &Vec<Vec<bool>>,
    words: &Vec<String>,
    pos: &Pos,
    initial_word: String,
    path: &Vec<Pos>,
) -> Vec<Word> {
    let x = usize::try_from(pos.x).unwrap();
    let y = usize::try_from(pos.y).unwrap();
    let current_word = initial_word + &board[y][x].to_string();
    let mut results: Vec<Word> = Vec::new();
    // Check exact matches
    for w in words.iter() {
        if *w == current_word {
            results.push(Word {
                path: path.to_vec(),
                word: w.to_string(),
            });
            break;
        }
    }
    for neighbor in get_neighbors(pos, &visited) {
        let mut inner_visited = visited.clone();
        let x = usize::try_from(neighbor.x).unwrap();
        let y = usize::try_from(neighbor.y).unwrap();
        let mut inner_path = path.clone();
        inner_path.push(Pos {
            x: neighbor.x,
            y: neighbor.y,
        });
        inner_visited[y][x] = true;
        let inner_word = current_word.clone();
        let mut inner_words: Vec<String> = Vec::new();
        for w in words.iter() {
            if w.starts_with(&current_word) {
                inner_words.push(w.to_string());
            }
        }
        if inner_words.len() > 0 {
            let mut neighbor_results = inner(
                board,
                &inner_visited,
                &inner_words,
                &neighbor,
                inner_word,
                &inner_path,
            );
            results.append(&mut neighbor_results)
        }
    }
    return results;
}

fn find_words_starting_from(
    board: &Vec<Vec<char>>,
    words: &Vec<String>,
    start_pos: Pos,
) -> Vec<Word> {
    let mut visited: Vec<Vec<bool>> = board.iter().map(|row| vec![false; row.len()]).collect();
    // Set starting position to visited
    let xusize = start_pos.x;
    let yusize = start_pos.y;
    visited[yusize][xusize] = true;
    let mut path: Vec<Pos> = Vec::new();
    path.push(start_pos.clone());
    let res = inner(board, &visited, &words, &start_pos, String::new(), &path);
    return res;
}

// Function to combine two vectors of solutions keeping only unique results
fn combine_results(a: &[Vec<Word1D>], b: &[Vec<Word1D>]) -> Vec<Vec<Word1D>> {
    let mut combined_results: Vec<Vec<Word1D>> = Vec::new();

    // Combine a and b into combined_results
    combined_results.extend_from_slice(a);
    combined_results.extend_from_slice(b);
    // Deduplicate the combined results
    combined_results.sort();
    combined_results.dedup();

    return combined_results;
}

fn intersects(a: &Word1D, b: &Word1D) -> bool {
    return (a & b) > 0;
}

fn add(a: &Word1D, b: &Word1D) -> Word1D {
    return a | b;
}

fn is_done(a: &Word1D) -> bool {
    // the board is 6x5 = 30 so full board equals 0011...11 as i32
    return Word1D::MAX >> 2 == *a;
}

fn find_solution(
    words: Vec<&Word1D>,
    solution: &Vec<&Word1D>,
    visited: &Word1D,
    max_result_count: usize,
) -> Vec<Vec<Word1D>> {
    if is_done(&visited) {
        let mut result_vec: Vec<Word1D> = Vec::new();
        for i in solution {
            result_vec.push(**i);
        }
        return vec![result_vec];
    }
    let mut words_left: Vec<&Word1D> = Vec::new();
    for i in 0..words.len() {
        if !intersects(visited, &words[i]) {
            words_left.push(&words[i]);
        }
    }
    if words_left.len() == 0usize {
        return Vec::new();
    }
    let mut results: Vec<Vec<Word1D>> = Vec::new();
    while !words_left.is_empty() {
        let word = words_left.pop().unwrap();
        let mut inner_solution = solution.clone();
        inner_solution.push(&word);
        let inner_visited = add(visited, word);
        let res = find_solution(
            words_left.clone(),
            &inner_solution,
            &inner_visited,
            max_result_count,
        );
        if res.is_empty() {
            continue;
        } else {
            results = combine_results(&results, &res);
        }
        if results.len() >= max_result_count {
            break;
        }
    }
    return results;
}

fn get_today_string() -> String {
    let now = Utc::now();
    let offset = FixedOffset::east_opt(10800).unwrap();
    let today: DateTime<FixedOffset> = DateTime::from_naive_utc_and_offset(now.naive_utc(), offset);
    let day = today.day();
    let month = today.month();
    let year = today.year();
    return String::from(day.to_string() + "." + &month.to_string() + "." + &year.to_string());
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut word_list_path: String =
        String::from("/Users/kaarlokock/projects/sanalouhos/sanalista.csv");
    let mut board_date: String =
        get_today_string();
    let mut max_count: usize = 1usize;

    if args.len() > 1 {
        word_list_path = args[1].to_string();
    }

    if args.len() > 2 {
        max_count = usize::from_str_radix(&args[2].to_string(), 10).unwrap_or_default();
    }

    if args.len() > 3 {
        board_date = args[3].to_string();
    }


    println!("Searching for {}", word_list_path);
    println!("In file {}", board_date);
    println!("Max results, {}", max_count);
    let read_res = read_csv(Path::new(&word_list_path));
    let words: Vec<String> = match read_res {
        Ok(data) => data.into_iter().map(|x| x.to_uppercase()).collect(),
        Err(err) => panic!("Problem opening file, {:?}", err),
    };
    println!("Found {:?} words from the static word list", words.len());
    let maybe_board_data = fetch_board_for_date(&board_date);
    println!("{:?}", maybe_board_data);
    let board = build_board(maybe_board_data);

    let mut matches: Vec<Word> = Vec::new();
    println!("Searching for all available words for this board");
    let word_search_start = Instant::now();
    for j in 0..board.len() {
        let row = &board[j];
        for i in 0..row.len() {
            let pos = Pos { x: i, y: j };
            let mut inner_matches = find_words_starting_from(&board, &words, pos);
            matches.append(&mut inner_matches);
        }
    }
    matches.sort_by_cached_key(|x| x.path.len());
    matches.reverse();

    let mut matches_1d: Vec<Word1D> = Vec::new();
    let rows = board.len();
    let columns = board.get(0).unwrap().len();
    for word in matches.clone() {
        let mut current = u32::MIN;
        for pos in word.path {
            let x = pos.x;
            let y = pos.y;
            let index = get_index(rows, columns, y, x);
            let mask = 1 << index;
            current = current | mask;
        }
        matches_1d.push(current);
    }
    let mut word_vectors: Vec<&Word1D> = Vec::new();
    for i in 0..matches_1d.len() {
        word_vectors.push(&matches_1d[i]);
    }
    println!("Words found, {:?}", matches_1d.len());
    let word_search_end = Instant::now();
    let word_search_duration = word_search_end.duration_since(word_search_start);
    println!(
        "word search took {} milliseconds",
        word_search_duration.as_millis()
    );
    let solution: Vec<&Word1D> = Vec::new();
    let solution_search_start = Instant::now();
    let res = find_solution(word_vectors, &solution, &u32::MIN, max_count);
    println!(
        "Solutions found in {} milliseconds",
        Instant::now().duration_since(solution_search_start).as_millis(),
    );
    for i in 0..res.len() {
        println!("Result #{:?}", i);
        for w in res[i].clone() {
            for m_idx in 0..matches_1d.len() {
                if w == matches_1d[m_idx] {
                    println!(" ## {} ## ", matches[m_idx].word);
                    break;
                }
            }

            print_board(&board, w);
        }
        print!("\n");
    }
}
