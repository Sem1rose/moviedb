use std::{cmp::Ordering, io::stdout};

use chrono::{DateTime, Local};
use itertools::Itertools;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        self, ExecutableCommand,
        event::{DisableMouseCapture, EnableMouseCapture},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    },
};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumCount, EnumDiscriminants, EnumIter, FromRepr, IntoStaticStr};
use tmdb::smo::TMDBDetailsResponse;
use trakt::smo::TraktDetailsResponse;

use crate::omdb::OMDBDetailsResponse;
pub use crate::pop_criterion;

pub type Term = Terminal<TermBackend>;
type TermBackend = CrosstermBackend<std::io::Stdout>;

pub fn initialize_terminal() -> anyhow::Result<Term> {
    set_panic_hook();

    crossterm::terminal::enable_raw_mode()?;

    let mut backend = TermBackend::new(stdout());
    backend.execute(EnterAlternateScreen)?;
    backend.execute(EnableMouseCapture)?;

    let mut term = Terminal::new(backend)?;
    term.hide_cursor()?;

    Ok(term)
}

pub fn reset_terminal(term: &mut Term) -> anyhow::Result<()> {
    term.backend_mut().execute(DisableMouseCapture)?;
    term.show_cursor()?;
    term.backend_mut().execute(LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        hook(info);
    }));
}

#[repr(usize)]
#[derive(Default, Clone, Copy, FromRepr, EnumCount, AsRefStr, EnumIter)]
#[strum(serialize_all = "title_case")]
pub enum Sort {
    #[default]
    MostRecent,
    DateAdded,
    UserRating,
    Rating,
    Name,
    ReleaseDate,
    Relevance,
}

#[derive(Serialize, Clone, Copy, Deserialize, Debug, PartialEq)]
pub enum Rating {
    Trakt(f64, u32),
    TMDB(f64, u32),
    IMDB(f64, u32),
}

impl From<Rating> for f64 {
    fn from(value: Rating) -> Self {
        match value {
            Rating::Trakt(rating, _) => rating,
            Rating::TMDB(rating, _) => rating,
            Rating::IMDB(rating, _) => rating,
        }
    }
}

impl PartialOrd for Rating {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(if matches!(self, Rating::IMDB(_, _)) {
            if matches!(other, Rating::IMDB(_, _)) {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        } else if matches!(self, Rating::Trakt(_, _)) {
            if matches!(other, Rating::IMDB(_, _)) {
                Ordering::Greater
            } else if matches!(other, Rating::Trakt(_, _)) {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        } else {
            if matches!(other, Rating::TMDB(_, _)) {
                Ordering::Equal
            } else {
                Ordering::Greater
            }
        })
    }
}

#[derive(Clone, EnumDiscriminants, Debug)]
#[strum_discriminants(derive(EnumIter, IntoStaticStr, FromRepr, EnumCount))]
#[strum_discriminants(repr(usize))]
pub enum FilterCriterion {
    #[strum_discriminants(strum(disabled))]
    Title(String, bool /*filter*/),
    Director(u32, bool /*inverted*/),
    Actors(Vec<u32>, bool /*contains all*/, bool /*inverted*/),
    Genres(
        Vec<String>,
        bool, /*contains all*/
        bool, /*inverted*/
    ),
    Released(
        u32,  /*lower bound*/
        u32,  /*upper bound*/
        bool, /*inverted*/
    ),
    DateAdded(
        u32,  /*lower bound*/
        u32,  /*upper bound*/
        bool, /*inverted*/
    ),
    RecentlyWatched(
        u32,  /*lower bound*/
        u32,  /*upper bound*/
        bool, /*inverted*/
    ),
    Rating(f64, Ordering, bool /*inverted*/),
    UserRating(f64, Ordering, bool /*inverted*/),
    Languages(Vec<String>, bool /*inverted*/),
    Country(String, bool /*inverted*/),
    Certification(Vec<Option<String>>, bool /*inverted*/),
}

#[macro_export]
macro_rules! pop_criterion(
    ($criteria:expr, $p:pat, $d:expr) => (
        {
            let position = $criteria.iter().position(|x| matches!(x, $p));
            if let Some(index) = position {
                $criteria.remove(index)
            } else {
                $d
            }
        }
    );
    ($criteria:expr, $p:pat) => (
        {
            let position = $criteria.iter().position(|x| matches!(x, $p));
            if let Some(index) = position {
                Some($criteria.remove(index))
            } else {
                None
            }
        }
    );
);

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct MovieID {
    pub tmdb: u32,
    pub imdb: String,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Movie {
    pub id:            MovieID,
    pub name:          String,
    pub year:          String,
    pub language:      String,
    pub ratings:       [Rating; 3],
    pub genres:        Vec<String>,
    pub collection:    Option<String>,
    pub collection_id: Option<u32>,
    pub overview:      String,
    pub runtime:       u32,
    pub released:      bool,
    pub tagline:       String,
    pub trailer:       Option<String>,
    pub plays:         Vec<(DateTime<Local>, f64)>,
}

impl From<TMDBDetailsResponse> for Movie {
    fn from(movie_details: TMDBDetailsResponse) -> Self {
        let mut collection = None;
        let mut collection_id = None;
        if let Some(belongs_to_collection) = movie_details.belongs_to_collection {
            collection = Some(belongs_to_collection.name);
            collection_id = Some(belongs_to_collection.id);
        }

        Self {
            name: movie_details.title,
            ratings: [
                Rating::TMDB(movie_details.vote_average, movie_details.vote_count),
                Rating::Trakt(0.0, 0),
                Rating::IMDB(0.0, 0),
            ],
            year: movie_details.release_date.split('-').collect_vec()[0].to_string(),
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
            plays: vec![],
        }
    }
}
impl From<TraktDetailsResponse> for Movie {
    fn from(movie_details: TraktDetailsResponse) -> Self {
        Self {
            name:          movie_details.title,
            ratings:       [
                Rating::TMDB(0.0, 0),
                Rating::Trakt(movie_details.rating, movie_details.votes),
                Rating::IMDB(0.0, 0),
            ],
            year:          movie_details.year.unwrap_or(1970).to_string(),
            language:      movie_details.language,
            id:            MovieID {
                tmdb: movie_details.ids.tmdb,
                imdb: movie_details.ids.imdb,
            },
            genres:        movie_details.genres,
            overview:      movie_details.overview,
            collection:    None,
            collection_id: None,
            runtime:       movie_details.runtime,
            released:      movie_details.status == "released",
            tagline:       movie_details.tagline,
            trailer:       movie_details.trailer,
            plays:         vec![],
        }
    }
}

impl Movie {
    pub fn add_tmdb_details(&mut self, tmdb_details: TMDBDetailsResponse) {
        let mut collection = None;
        let mut collection_id = None;
        if let Some(belongs_to_collection) = tmdb_details.belongs_to_collection {
            collection = Some(belongs_to_collection.name);
            collection_id = Some(belongs_to_collection.id);
        }

        self.collection = collection;
        self.collection_id = collection_id;
        self.ratings[2] = Rating::TMDB(tmdb_details.vote_average, tmdb_details.vote_count);
    }

    pub fn add_trakt_details(&mut self, trakt_details: TraktDetailsResponse) {
        self.ratings[1] = Rating::Trakt(trakt_details.rating, trakt_details.votes);
        self.trailer = trakt_details.trailer;
    }

    pub fn add_omdb_details(&mut self, omdb_details: OMDBDetailsResponse) {
        self.ratings[0] = Rating::IMDB(
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

    pub fn get_external_rating(&self) -> f64 {
        for _r in self
            .ratings
            .iter()
            .sorted_by(|&&a, &b| a.partial_cmp(b).unwrap())
        {
            match _r {
                Rating::IMDB(r, _) | Rating::Trakt(r, _) | Rating::TMDB(r, _) if *r > 0.0 => {
                    return *r;
                }
                _ => (),
            }
        }
        f64::NAN
    }

    pub fn get_user_rating(&self) -> f64 {
        self.plays.last().map(|x| x.1).unwrap_or(0.0)
    }

    pub fn get_latest_play(&self) -> DateTime<Local> {
        self.plays
            .last()
            .map(|x| x.0)
            .unwrap_or(DateTime::default())
    }

    pub fn get_first_play(&self) -> DateTime<Local> {
        self.plays
            .first()
            .map(|x| x.0)
            .unwrap_or(DateTime::default())
    }

    pub fn add_play(&mut self, date: DateTime<Local>, rating: f64) {
        self.plays.push((date, rating));
    }
}

impl std::cmp::PartialEq<Movie> for Movie {
    fn eq(&self, other: &Movie) -> bool {
        self.id.imdb == other.id.imdb
    }
}

impl std::cmp::PartialOrd<Movie> for Movie {
    fn partial_cmp(&self, other: &Movie) -> Option<Ordering> {
        let mut rating_a: f64 = f64::NAN;
        let mut rating_b: f64 = f64::NAN;

        for i in (0..self.ratings.len()).rev() {
            if let Rating::IMDB(r_a, c_a) = self.ratings[i] {
                if let Rating::IMDB(r_b, c_b) = other.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
            if let Rating::Trakt(r_a, c_a) = self.ratings[i] {
                if let Rating::Trakt(r_b, c_b) = other.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
            if let Rating::TMDB(r_a, c_a) = self.ratings[i] {
                if let Rating::TMDB(r_b, c_b) = other.ratings[i] {
                    if r_a == 0.0 || r_b == 0.0 {
                        continue;
                    }

                    if r_a != r_b {
                        rating_a = r_a;
                        rating_b = r_b;
                    } else {
                        rating_a = c_a as f64;
                        rating_b = c_b as f64;
                    }

                    break;
                }
            }
        }

        rating_a.partial_cmp(&rating_b)
    }
}

#[derive(Serialize, Deserialize)]
pub struct OldMovie {
    pub id:            MovieID,
    pub name:          String,
    pub year:          String,
    pub language:      String,
    pub ratings:       [Rating; 3],
    pub genres:        Vec<String>,
    pub collection:    Option<String>,
    pub collection_id: Option<u32>,
    pub overview:      String,
    pub runtime:       u32,
    pub released:      bool,
    pub tagline:       String,
    pub trailer:       Option<String>,
    pub plays:         Vec<(DateTime<Local>, f64)>,
}

impl From<OldMovie> for Movie {
    fn from(value: OldMovie) -> Self {
        Self {
            name:          value.name,
            ratings:       value.ratings,
            year:          value.year,
            language:      value.language,
            id:            value.id,
            genres:        value.genres,
            overview:      value.overview,
            collection:    value.collection,
            collection_id: value.collection_id,
            runtime:       value.runtime,
            released:      value.released,
            tagline:       value.tagline,
            trailer:       value.trailer,
            plays:         value.plays,
        }
    }
}
