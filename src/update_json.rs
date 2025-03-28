// use crate::{
//     app::{Config, Movie, Result},
//     config_tmdb::TMDBConfig,
//     config_trakt::TraktConfig,
//     tmdb, trakt,
// };
// use std::fs;

// pub fn change_ratings() -> Result<()> {
//     let mut config = Config::new();
//     config.init_dirs()?;

//     let file_path = config.dirs.home.join("ratings.json");

//     let file_contents = fs::read_to_string(&file_path)
//         .unwrap_or_else(|_| panic!("Couldn't read database contents at {}", file_path.display()));
//     let json_contents = json::parse(&file_contents).unwrap_or_else(|_| {
//         panic!(
//             "Couldn't parse database contents at {}",
//             file_path.display()
//         )
//     });

//     let user_ratings = json_contents
//         .members()
//         .map(|x| {
//             x["user_rating"]
//                 .to_string()
//                 .parse()
//                 .expect("couldn't parse ratings!")
//         })
//         .collect::<Vec<f64>>();

//     let mut tmdb_config = TMDBConfig::new();
//     tmdb_config.init(&config)?;

//     let mut trakt_config = TraktConfig::new();
//     trakt_config.init(&config)?;

//     tmdb::populate_tokens(&config, &mut tmdb_config)?;
//     trakt::populate_tokens(&config, &mut trakt_config)?;

//     let mut out: Vec<Movie> = vec![];

//     let titles = json_contents
//         .members()
//         .map(|x| x["name"].to_string())
//         .collect::<Vec<String>>();
//     let movies = json_contents
//         .members()
//         .map(|x| x["id"].to_string().parse().unwrap())
//         .collect::<Vec<u32>>();

//     for (i, x) in movies.iter().enumerate() {
//         println!("{}", titles[i]);

//         let tmdb_movie_details =
//             tmdb::get_movie_details(&tmdb_config, *x).expect("couldn't get movie details");
//         let trakt_movie_details =
//             trakt::get_movie_details(&trakt_config, &tmdb_movie_details.imdb_id)
//                 .expect("couldn't get movie details");

//         out.push(
//             Movie::from(tmdb_movie_details, user_ratings[i]).add_trakt_details(trakt_movie_details),
//         );
//     }

//     // let movies = json_contents
//     //     .members()
//     //     .map(|x| x["name"].to_string())
//     //     .collect::<Vec<String>>();
//     // for (i, x) in movies.iter().enumerate() {
//     //     let x = tmdb::find_movie(&config, x.as_str())
//     //         .unwrap_or_else(|_| panic!("couldn't get for movie {x}"));
//     //     for y in x.iter() {
//     //         println!(
//     //             "{} - {} - {} - {}",
//     //             y.id, y.title, y.release_date, y.vote_average
//     //         );
//     //     }

//     //     print!("Choose: ");
//     //     let _ = stdout().flush();

//     //     let mut input = String::new();
//     //     let mut j: u32 = 0;

//     //     if stdin().read_line(&mut input).is_ok() && input.trim() != "" {
//     //         if let Some('\n') = input.chars().next_back() {
//     //             input.pop();
//     //         }
//     //         if let Some('\r') = input.chars().next_back() {
//     //             input.pop();
//     //         }

//     //         j = input.parse().expect("couldn't parse input");
//     //     }
//     //     let movie_details = tmdb::get_movie_details(&config, x[j as usize].id as u32)
//     //         .expect("couldn't get movie details");

//     //     out.push(Movie::new(
//     //         movie_details.title,
//     //         user_ratings[i],
//     //         movie_details.vote_average,
//     //         movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
//     //         movie_details.id,
//     //         movie_details
//     //             .genres
//     //             .iter()
//     //             .map(|x| x.name.to_string())
//     //             .collect(),
//     //         movie_details.overview,
//     //         if movie_details.belongs_to_collection.is_some() {
//     //             Some(movie_details.belongs_to_collection.unwrap().name)
//     //         } else {
//     //             None
//     //         },
//     //         movie_details.runtime,
//     //         movie_details.status == "Released",
//     //         movie_details.tagline,
//     //         movie_details.vote_count,
//     //     ));
//     // }

//     let string = serde_json::to_string_pretty(out.as_slice())?;

//     fs::write(config.dirs.home.join("ratings3.json"), string)?;

//     Ok(())
// }
