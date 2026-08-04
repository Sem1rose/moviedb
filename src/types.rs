use std::{cmp::Ordering, io::stdout};

use chrono::{DateTime, Local};
use punch_play::smo::PunchPlayDetailsResponse;
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
use tmdb::smo::TMDBMovieDetails;

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

#[derive(Serialize, Clone, Copy, Deserialize, Debug, Default)]
pub struct ExternalRatings {
    pub imdb:       (f64, u32),
    pub trakt:      (f64, u32),
    pub letterboxd: (f64, u32),
    pub tmdb:       (f64, u32),
    pub popcorn:    (u32, u32),
    pub tomatoes:   (u32, u32),
}

#[derive(Clone, EnumDiscriminants, Debug)]
#[strum_discriminants(derive(EnumIter, IntoStaticStr, EnumCount))]
#[strum_discriminants(repr(usize))]
pub enum FilterCriterion {
    // #[strum_discriminants(strum(disabled))]
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
    FirstWatched(
        u32,  /*lower bound*/
        u32,  /*upper bound*/
        bool, /*inverted*/
    ),
    LastWatched(
        u32,  /*lower bound*/
        u32,  /*upper bound*/
        bool, /*inverted*/
    ),
    UserRating(f64, Ordering, bool /*inverted*/),
    Rating(f64, Ordering, bool /*inverted*/),
    Language(Vec<String>, bool /*inverted*/),
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
pub struct HistoryEntry {
    pub date:   DateTime<Local>,
    pub rating: f64,
    pub note:   Option<String>,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Entry {
    pub movie_id: MovieID,
    pub history:  Vec<HistoryEntry>,
}
impl Entry {
    pub fn get_user_rating(&self) -> f64 {
        self.history.last().map(|x| x.rating).unwrap_or(0.0)
    }

    pub fn get_latest_play(&self) -> DateTime<Local> {
        self.history
            .last()
            .map(|x| x.date)
            .unwrap_or(DateTime::default())
    }

    pub fn get_first_play(&self) -> DateTime<Local> {
        self.history
            .first()
            .map(|x| x.date)
            .unwrap_or(DateTime::default())
    }

    pub fn add_play(&mut self, date: DateTime<Local>, rating: f64, note: Option<String>) {
        self.history.push(HistoryEntry { date, rating, note });
    }
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Role {
    pub id:               u32,
    pub job_or_character: String,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Person {
    pub id:     u32,
    pub gender: usize,
    pub name:   String,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Collection {
    pub id:       u32,
    pub name:     String,
    pub overview: String,
    pub parts:    Vec<u32>,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct OldCollection {
    pub id:       u32,
    pub name:     String,
    pub overview: String,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Credits {
    pub cast: Vec<Role>,
    pub crew: Vec<Role>,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct MovieID {
    pub tmdb: u32,
    pub imdb: String,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Movie {
    pub id:               MovieID,
    pub title:            String,
    pub release_date:     DateTime<Local>,
    pub language:         String,
    pub external_ratings: ExternalRatings,
    pub genres:           Vec<String>,
    pub tmdb_collection:  Option<u32>,
    pub overview:         String,
    pub runtime:          u32,
    pub released:         bool,
    pub tagline:          String,
    pub certification:    String,
    pub origin_country:   String,
    pub credits:          Credits,
    pub recommendations:  Vec<u32>,
}
impl From<&TMDBMovieDetails> for Movie {
    fn from(tmdb_details: &TMDBMovieDetails) -> Self {
        let (cast, crew) = if let Some(credits) = tmdb_details.credits.as_ref() {
            (
                credits
                    .cast
                    .iter()
                    .take(20)
                    .map(|x| Role {
                        id:               x.id,
                        job_or_character: x.character.clone().unwrap_or("Unknown".into()),
                    })
                    .collect(),
                credits
                    .crew
                    .iter()
                    .filter(|x| {
                        ["Director", "Original Music Composer", "Additional Music"]
                            .contains(&x.job.as_ref().unwrap().as_str())
                    })
                    .map(|x| Role {
                        id:               x.id,
                        job_or_character: x.job.clone().unwrap_or("Unknown".into()),
                    })
                    .collect(),
            )
        } else {
            (vec![], vec![])
        };
        Self {
            title:            tmdb_details.title.clone(),
            external_ratings: ExternalRatings {
                tmdb: (tmdb_details.vote_average, tmdb_details.vote_count),
                ..Default::default()
            },
            release_date:     DateTime::from_timestamp_millis(0)
                .unwrap()
                .with_timezone(&Local), //movie_details.release_date.split('-').collect_vec()[0].to_string(),
            language:         tmdb_details.original_language.clone(),
            id:               MovieID {
                tmdb: tmdb_details.id,
                imdb: tmdb_details.imdb_id.clone(),
            },
            genres:           tmdb_details
                .genres
                .iter()
                .map(|x| x.name.to_string())
                .collect(),
            overview:         tmdb_details.overview.clone(),
            tmdb_collection:  tmdb_details.belongs_to_collection.clone().map(|x| x.id),
            runtime:          tmdb_details.runtime,
            released:         tmdb_details.status == "Released",
            tagline:          tmdb_details.tagline.clone(),
            certification:    tmdb_details.certificate.clone().unwrap(),
            origin_country:   tmdb_details
                .origin_country
                .clone()
                .map(|x| x.get(0).unwrap_or(&"Unknown".into()).clone())
                .unwrap_or("Unknown".into()),
            credits:          Credits { cast, crew },
            recommendations:  tmdb_details.recommendations.clone().unwrap_or_default(),
        }
    }
}

// #[derive(Serialize, Clone, Deserialize, Debug)]
// pub struct Movie {
//     pub id:               MovieID,
//     pub name:             String,
//     pub year:             String,
//     pub language:         String,
//     pub external_ratings: ExternalRatings,
//     pub genres:           Vec<String>,
//     pub collection:       Option<String>,
//     pub collection_id:    Option<u32>,
//     pub overview:         String,
//     pub runtime:          u32,
//     pub released:         bool,
//     pub tagline:          String,
//     pub trailer:          Option<String>,
//     pub plays:            Vec<(DateTime<Local>, f64)>,
// }

// impl From<TMDBMovieDetails> for Movie {
//     fn from(movie_details: TMDBMovieDetails) -> Self {
//         let mut collection = None;
//         let mut collection_id = None;
//         if let Some(belongs_to_collection) = movie_details.belongs_to_collection {
//             collection = Some(belongs_to_collection.name);
//             collection_id = Some(belongs_to_collection.id);
//         }

//         Self {
//             name: movie_details.title,
//             external_ratings: ExternalRatings {
//                 tmdb: (movie_details.vote_average, movie_details.vote_count),
//                 ..Default::default()
//             },
//             year: movie_details.release_date.split('-').collect_vec()[0].to_string(),
//             language: movie_details.original_language,
//             id: MovieID {
//                 tmdb: movie_details.id,
//                 imdb: movie_details.imdb_id,
//             },
//             genres: movie_details
//                 .genres
//                 .iter()
//                 .map(|x| x.name.to_string())
//                 .collect(),
//             overview: movie_details.overview,
//             collection,
//             collection_id,
//             runtime: movie_details.runtime,
//             released: movie_details.status == "Released",
//             tagline: movie_details.tagline,
//             trailer: None,
//             plays: vec![],
//         }
//     }
// }
// impl From<TraktDetailsResponse> for Movie {
//     fn from(movie_details: TraktDetailsResponse) -> Self {
//         Self {
//             name:             movie_details.title,
//             external_ratings: ExternalRatings {
//                 trakt: (movie_details.rating, movie_details.votes),
//                 ..Default::default()
//             },
//             year:             movie_details.year.unwrap_or(1970).to_string(),
//             language:         movie_details.language,
//             id:               MovieID {
//                 tmdb: movie_details.ids.tmdb,
//                 imdb: movie_details.ids.imdb,
//             },
//             genres:           movie_details.genres,
//             overview:         movie_details.overview,
//             collection:       None,
//             collection_id:    None,
//             runtime:          movie_details.runtime,
//             released:         movie_details.status == "released",
//             tagline:          movie_details.tagline,
//             trailer:          movie_details.trailer,
//             plays:            vec![],
//         }
//     }
// }

impl Movie {
    // pub fn add_tmdb_details(&mut self, tmdb_details: TMDBMovieDetails) {
    //     let mut collection = None;
    //     let mut collection_id = None;
    //     if let Some(belongs_to_collection) = tmdb_details.belongs_to_collection {
    //         collection = Some(belongs_to_collection.name);
    //         collection_id = Some(belongs_to_collection.id);
    //     }

    //     self.collection = collection;
    //     self.collection_id = collection_id;
    //     self.external_ratings.tmdb = (tmdb_details.vote_average, tmdb_details.vote_count);
    // }

    // pub fn add_trakt_details(&mut self, trakt_details: TraktDetailsResponse) {
    //     self.external_ratings.trakt = (trakt_details.rating, trakt_details.votes);
    // }

    pub fn add_punch_play_details(&mut self, punch_play_details: PunchPlayDetailsResponse) {
        if let Some(external_ratings) = punch_play_details.external_ratings {
            for external_rating in external_ratings.ratings {
                if let Some(source) = external_rating.source.as_ref() {
                    if source == "imdb" {
                        self.external_ratings.imdb = (
                            external_rating.value.unwrap_or(0.0),
                            external_rating.votes.unwrap_or(0),
                        );
                    } else if source == "trakt" {
                        self.external_ratings.trakt = (
                            external_rating.value.unwrap_or(0.0),
                            external_rating.votes.unwrap_or(0),
                        );
                    } else if source == "letterboxd" {
                        self.external_ratings.letterboxd = (
                            external_rating.value.unwrap_or(0.0),
                            external_rating.votes.unwrap_or(0),
                        );
                    } else if source == "popcorn" {
                        self.external_ratings.popcorn = (
                            external_rating.value.unwrap_or(0.0) as u32,
                            external_rating.votes.unwrap_or(0),
                        );
                    } else if source == "tomatoes" {
                        self.external_ratings.tomatoes = (
                            external_rating.value.unwrap_or(0.0) as u32,
                            external_rating.votes.unwrap_or(0),
                        );
                    }
                }
            }
        }
    }

    pub fn add_omdb_details(&mut self, omdb_details: OMDBDetailsResponse) {
        self.external_ratings.imdb = (
            omdb_details.imdb_rating.parse().unwrap_or(0.0),
            omdb_details
                .imdb_votes
                .chars()
                .filter(|char| char.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0),
        );
    }

    pub fn get_external_rating(&self) -> f64 {
        if self.external_ratings.imdb.0 > 0.0 {
            self.external_ratings.imdb.0
        } else if self.external_ratings.trakt.0 > 0.0 {
            self.external_ratings.trakt.0
        } else if self.external_ratings.letterboxd.0 > 0.0 {
            self.external_ratings.letterboxd.0 * 2.0
        } else if self.external_ratings.tmdb.0 > 0.0 {
            self.external_ratings.tmdb.0
        } else if self.external_ratings.popcorn.0 > 0 {
            self.external_ratings.popcorn.0 as f64 / 10.0
        } else if self.external_ratings.tomatoes.0 > 0 {
            self.external_ratings.tomatoes.0 as f64 / 10.0
        } else {
            f64::NAN
        }
    }
}

impl std::cmp::PartialEq<Movie> for Movie {
    fn eq(&self, other: &Movie) -> bool {
        self.id.imdb == other.id.imdb
    }
}

impl std::cmp::PartialOrd<Movie> for Movie {
    fn partial_cmp(&self, other: &Movie) -> Option<Ordering> {
        macro_rules! cmp_rating {
            ($field:ident) => {
                if self.external_ratings.$field.0 as f64 != 0.0
                    && other.external_ratings.$field.0 as f64 == 0.0
                {
                    if self.external_ratings.$field.0 != other.external_ratings.$field.0 {
                        return self
                            .external_ratings
                            .$field
                            .0
                            .partial_cmp(&other.external_ratings.$field.0);
                    } else {
                        return self
                            .external_ratings
                            .$field
                            .1
                            .partial_cmp(&other.external_ratings.$field.1);
                    }
                }
            };
        }

        cmp_rating!(imdb);
        cmp_rating!(trakt);
        cmp_rating!(letterboxd);
        cmp_rating!(tmdb);
        cmp_rating!(popcorn);
        cmp_rating!(tomatoes);

        unreachable!()
    }
}

// #[derive(Serialize, Deserialize)]
// pub struct OldMovie {
//     pub id:            MovieID,
//     pub name:          String,
//     pub year:          String,
//     pub language:      String,
//     // pub ratings:       ExternalRatings,
//     pub genres:        Vec<String>,
//     pub collection:    Option<String>,
//     pub collection_id: Option<u32>,
//     pub overview:      String,
//     pub runtime:       u32,
//     pub released:      bool,
//     pub tagline:       String,
//     pub trailer:       Option<String>,
//     pub plays:         Vec<(DateTime<Local>, f64)>,
// }

// impl From<OldMovie> for Movie {
//     fn from(value: OldMovie) -> Self {
//         Self {
//             name:             value.name,
//             external_ratings: Default::default(),
//             year:             value.year,
//             language:         value.language,
//             id:               value.id,
//             genres:           value.genres,
//             overview:         value.overview,
//             collection:       value.collection,
//             collection_id:    value.collection_id,
//             runtime:          value.runtime,
//             released:         value.released,
//             tagline:          value.tagline,
//             trailer:          value.trailer,
//             plays:            value.plays,
//         }
//     }
// }
