use crate::app::Movie;
use std::{env, fs, path::Path};

pub fn fetch_movies() -> Option<Vec<Movie>> {
    let file_path = Path::new(&env::var("HOME").ok().unwrap()).join("Videos/ratings.json");

    let file_contents = fs::read_to_string(&file_path)
        .unwrap_or_else(|_| panic!("Couldn't read database contents at {}", file_path.display()));
    let json_contents = json::parse(&file_contents).unwrap_or_else(|_| {
        panic!(
            "Couldn't parse database contents at {}",
            file_path.display()
        )
    });

    let movies = json_contents
        .members()
        .map(|x| {
            Movie::new(
                x["name"].to_string(),
                x["rating"]
                    .to_string()
                    .parse::<f32>()
                    .expect("Rating was not a number!"),
                x["url"].to_string(),
                {
                    let split_pos = x["url"].to_string().char_indices().nth_back(3).unwrap().0;
                    String::from(&x["url"].to_string()[split_pos..])
                },
                0,
            )
        })
        .collect::<Vec<Movie>>();
    Some(movies)
}
