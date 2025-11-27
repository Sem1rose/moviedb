use crate::{
    config::{
        config_omdb::OMDBConfig, config_tmdb::TMDBConfig, config_trakt::TraktConfig, Config,
        Credentials,
    },
    omdb::{self},
    tmdb::{self},
    trakt::{self},
    types::*,
};
use serde::Deserialize;
use std::{
    fs::{self, read_to_string},
    sync::mpsc::channel,
};

pub fn update_movies() -> anyhow::Result<()> {
    let mut config = Config::default();
    config.init_dirs()?;

    let file_path = config.dirs.home.join("ratings.json");

    let file_contents =
        read_to_string(".credentials").expect("Couldn't read credentials from .credentials!");
    let creds: Credentials = serde_json::from_str(&file_contents)
        .expect("Couldn't deserialize credentials at .credentials");

    let (tx, _) = channel();
    let mut tmdb_config = TMDBConfig::new(tx);
    tmdb_config.set_access_token(creds.tmdb_access_token);
    tmdb_config.set_creds(TMDBConfig::read_creds(&config).expect("error reading tmdb creds"))?;

    let (tx, _) = channel();
    let mut trakt_config = TraktConfig::new(tx);
    trakt_config.set_secrets(creds.trakt_client_id, creds.trakt_client_secret);
    trakt_config.set_creds(TraktConfig::read_creds(&config).expect("error reading trakt creds"))?;

    let mut omdb_config = OMDBConfig::new();
    omdb_config.set_key(creds.omdb_key);

    // let mut out: Vec<Movie> = vec![];

    let file_contents = fs::read_to_string(&file_path)
        .unwrap_or_else(|_| panic!("Couldn't read database contents at {}", file_path.display()));
    let mut movies: Vec<Movie> = serde_json::from_str(&file_contents).unwrap();
    let mut ids = vec![];
    for (i, movie) in movies.iter().enumerate() {
        if let Rating::IMDB(0.0, 0) = movie.ratings[3] {
            ids.push(i);
        } else if let Rating::Metascore(0) = movie.ratings[2] {
            ids.push(i);
        }
    }

    for (i, id) in ids.iter().enumerate() {
        println!("{}/{}: {}", i + 1, ids.len(), movies[*id].name);

        let omdb_result = omdb::get_movie_details(&omdb_config, &movies[*id].id.imdb)
            .map(Some)
            .unwrap_or(None);

        if let Some(omdb) = omdb_result {
            println!("\tgot omdb");
            movies[*id].add_omdb_details(omdb);
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    // let json_contents = json::parse(&file_contents).unwrap_or_else(|_| {
    //     panic!(
    //         "Couldn't parse database contents at {}",
    //         file_path.display()
    //     )
    // });

    // let titles = json_contents
    //     .members()
    //     .map(|x| x["name"].to_string())
    //     .collect::<Vec<_>>();
    // let ratings = json_contents
    //     .members()
    //     .map(|x| x["user_rating"].to_string().parse().unwrap())
    //     .collect::<Vec<f64>>();
    // let tmdb_ids = json_contents
    //     .members()
    //     .map(|x| x["id"]["tmdb"].to_string().parse().unwrap())
    //     .collect::<Vec<u32>>();

    // for i in 0..tmdb_ids.len() {
    //     println!("{}/{}: {}", i + 1, tmdb_ids.len(), titles[i]);

    //     let tmdb_result = tmdb::get_movie_details(&tmdb_config, tmdb_ids[i]);
    //     println!("\tgot tmdb");

    //     if let Ok(tmdb_response) = tmdb_result {
    //         let trakt_result = trakt::get_movie_details(&trakt_config, &tmdb_response.imdb_id)
    //             .map(Some)
    //             .unwrap_or(None);
    //         println!("\tgot trakt");
    //         let omdb_result = omdb::get_movie_details(&omdb_config, &tmdb_response.imdb_id)
    //             .map(Some)
    //             .unwrap_or(None);
    //         println!("\tgot omdb");

    //         let mut movie = Movie::from(tmdb_response, ratings[i]);
    //         if let Some(trakt) = trakt_result {
    //             movie.add_trakt_details(trakt);
    //         }
    //         if let Some(omdb) = omdb_result {
    //             movie.add_omdb_details(omdb);
    //         }

    //         println!("\tdone");
    //         out.push(movie);
    //     } else if let Err(error) = tmdb_result {
    //         panic!("\t Error: {}", error);
    //     }
    // }

    let string = serde_json::to_string_pretty(movies.as_slice())?;

    fs::write(config.dirs.home.join("new_ratings2.json"), string)?;

    Ok(())
}
