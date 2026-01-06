use crate::{omdb::OMDBDetailsResponse, tmdb::TMDBDetailsResponse, trakt::TraktDetailsResponse};
use chrono::{DateTime, Local};
use ratatui::{
    crossterm::{self, terminal::EnterAlternateScreen, ExecutableCommand},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::io::stdout;

// pub type OptionalResult<T> = anyhow::Result<T, Option<anyhow::Error>>;
type TermBackend = CrosstermBackend<std::io::Stdout>;
pub type Term = Terminal<TermBackend>;

pub fn initialize_terminal() -> anyhow::Result<Term> {
    set_panic_hook();

    crossterm::terminal::enable_raw_mode()?;

    let mut backend = TermBackend::new(stdout());
    backend.execute(EnterAlternateScreen)?;

    let mut term = Terminal::new(backend)?;
    term.hide_cursor()?;

    Ok(term)
}

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        hook(info);
    }));
}

#[derive(Serialize, Clone, Copy, Deserialize, Debug)]
pub enum Rating {
    Trakt(f64, u32),
    TMDB(f64, u32),
    IMDB(f64, u32),
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct MovieID {
    pub tmdb: u32,
    pub imdb: String,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Movie {
    pub id: MovieID,
    pub name: String,
    pub year: String,
    pub language: String,
    pub ratings: [Rating; 3],
    pub genres: Vec<String>,
    pub collection: Option<String>,
    pub collection_id: Option<u32>,
    pub overview: String,
    pub runtime: u32,
    pub released: bool,
    pub tagline: String,
    pub trailer: Option<String>,
    pub plays: Vec<(DateTime<Local>, f64)>,
}

impl Movie {
    pub fn from(movie_details: TMDBDetailsResponse, user_rating: f64) -> Self {
        let mut collection: Option<String> = None;
        let mut collection_id: Option<u32> = None;
        if movie_details.belongs_to_collection.is_some() {
            collection = Some(movie_details.belongs_to_collection.clone().unwrap().name);
            collection_id = Some(movie_details.belongs_to_collection.clone().unwrap().id);
        }

        Self {
            name: movie_details.title,
            ratings: [
                Rating::TMDB(movie_details.vote_average, movie_details.vote_count),
                Rating::Trakt(0.0, 0),
                Rating::IMDB(0.0, 0),
            ],
            year: movie_details.release_date.split('-').collect::<Vec<_>>()[0].to_string(),
            language: movie_details.original_language,
            id: MovieID {
                tmdb: movie_details.id,
                imdb: movie_details.imdb_id,
            },
            genres: movie_details
                .genres
                .iter()
                .map(|x| x.name.to_string())
                .collect(),
            overview: movie_details.overview,
            collection,
            collection_id,
            runtime: movie_details.runtime,
            released: movie_details.status == "Released",
            tagline: movie_details.tagline,
            trailer: None,
            plays: vec![(Local::now(), user_rating)],
        }
    }

    pub fn add_trakt_details(&mut self, trakt_details: TraktDetailsResponse) {
        self.ratings[1] = Rating::Trakt(trakt_details.rating, trakt_details.votes);
        self.trailer = trakt_details.trailer;
    }
    pub fn add_omdb_details(&mut self, omdb_details: OMDBDetailsResponse) {
        self.ratings[2] = Rating::IMDB(
            omdb_details.imdbRating.parse().unwrap_or(0.0),
            omdb_details
                .imdbVotes
                .chars()
                .filter(|char| char.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0),
        );
    }

    pub fn get_user_rating(&self) -> f64 {
        self.plays.last().map(|x| x.1).unwrap_or(0.0)
    }

    pub fn add_play(&mut self, datetime: DateTime<Local>, rating: f64) {
        self.plays.push((datetime, rating));
    }
    pub fn edit_user_rating(&mut self, new_rating: f64) {
        self.plays.last_mut().map(|x| x.1 = new_rating);
    }
}

impl std::cmp::PartialEq<&Movie> for Movie {
    fn eq(&self, other: &&Movie) -> bool {
        self.id.imdb == other.id.imdb
    }
}
impl std::cmp::PartialEq<&Movie> for &Movie {
    fn eq(&self, other: &&Movie) -> bool {
        self.id.imdb == other.id.imdb
    }
}

#[derive(Serialize, Deserialize)]
pub struct OldMovie {
    pub id: MovieID,
    pub name: String,
    pub year: String,
    pub user_rating: f64,
    pub language: String,
    pub ratings: [Rating; 3],
    pub genres: Vec<String>,
    pub collection: Option<String>,
    pub collection_id: Option<u32>,
    pub overview: String,
    pub runtime: u32,
    pub released: bool,
    pub tagline: String,
    pub trailer: Option<String>,
    pub plays: Vec<(DateTime<Local>, f64)>,
}

impl From<OldMovie> for Movie {
    fn from(value: OldMovie) -> Self {
        Self {
            name: value.name,
            ratings: value.ratings,
            year: value.year,
            language: value.language,
            id: value.id,
            genres: value.genres,
            overview: value.overview,
            collection: value.collection,
            collection_id: value.collection_id,
            runtime: value.runtime,
            released: value.released,
            tagline: value.tagline,
            trailer: value.trailer,
            plays: vec![(
                DateTime::from_timestamp(0, 0)
                    .unwrap()
                    .with_timezone(&Local),
                value.user_rating,
            )],
        }
    }
}
