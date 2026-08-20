use std::{cmp::Ordering, io::stdout};

use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use indexmap::IndexMap;
use log::info;
use punch_play::smo::{DetailsResponse, HistoryItem as PunchPlayHistoryItem};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        self, ExecutableCommand,
        event::{
            DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        },
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    },
};
use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize};
use strum::{
    AsRefStr, EnumCount, EnumDiscriminants, EnumIter, FromRepr, IntoEnumIterator, IntoStaticStr,
};
use tmdb::smo::{MovieDetails, Person as TMDBPerson};
use trakt::smo::TraktDetailsResponse;

use crate::omdb::OMDBDetailsResponse;
pub use crate::pop_criterion;

pub type Term = Terminal<TermBackend>;
type TermBackend = CrosstermBackend<std::io::Stdout>;
pub type FxIndexMap<K, V> = IndexMap<K, V, FxBuildHasher>;
pub type BoxedFn<T, R> = Box<dyn Fn(&T) -> R>;
pub type BoxedMutFn<T, R> = Box<dyn Fn(&mut T) -> R>;

pub fn initialize_terminal() -> anyhow::Result<Term> {
    set_panic_hook();

    crossterm::terminal::enable_raw_mode()?;

    let mut backend = TermBackend::new(stdout());
    backend.execute(crossterm::cursor::Hide)?;
    backend.execute(EnterAlternateScreen)?;
    backend.execute(EnableBracketedPaste)?;
    backend.execute(EnableMouseCapture)?;

    Ok(Terminal::new(backend)?)
}

pub fn try_restore_terminal() -> anyhow::Result<()> {
    execute!(stdout(), DisableMouseCapture)?;
    execute!(stdout(), DisableBracketedPaste)?;
    execute!(stdout(), LeaveAlternateScreen)?;
    execute!(stdout(), crossterm::cursor::Show)?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Err(err) = try_restore_terminal() {
            eprintln!("Failed to restore terminal: {err}");
        }

        hook(info);
    }));
}

pub struct MovieDetailsResponse {
    pub tmdb:       Option<MovieDetails>,
    pub trakt:      Option<TraktDetailsResponse>,
    pub punch_play: Option<DetailsResponse>,
    pub omdb:       Option<OMDBDetailsResponse>,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Hash, Eq, Default, Debug)]
pub enum ListID {
    #[default]
    Watched,
    Watchlist,
    TMDB(u32),
    Local(u32),
    PunchPlay(u32),
    Collection(u32),
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ListItem {
    pub id:       u32,
    pub added_at: DateTime<Utc>,
}
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct List {
    pub id:       ListID,
    pub name:     String,
    pub items:    Vec<ListItem>,
    pub readonly: bool,
}
// impl From<&[Entry]> for List {
//     fn from(value: &[Entry]) -> Self {
//         Self {
//             id:     Default::default(),
//             name:   "Watched Movies".into(),
//             items: value.iter().map(|x| x.movie_id).collect(),
//             readonly: false
//         }
//     }
// }
impl List {
    pub fn from_tmdb(value: tmdb::list::smo::ListDetails, readonly: bool) -> Self {
        Self {
            id: ListID::TMDB(value.id),
            name: value.name,
            items: value
                .items
                .unwrap_or_default()
                .iter()
                .filter_map(|x| {
                    (x.media_type.as_ref().unwrap() == "movie").then_some(ListItem {
                        id:       x.id,
                        added_at: Default::default(),
                    })
                })
                .collect(),
            readonly,
        }
    }

    pub fn from_punch_play(value: punch_play::list::smo::ListDetails, readonly: bool) -> Self {
        Self {
            id: ListID::PunchPlay(value.id),
            name: value.name,
            items: value
                .items
                .unwrap_or_default()
                .iter()
                .filter_map(|x| {
                    (x.item_type == "movie").then_some(ListItem {
                        id:       x.tmdb_id,
                        added_at: x.added_at,
                    })
                })
                .collect(),
            readonly,
        }
    }

    pub fn from_collection(value: tmdb::collection::smo::CollectionDetails) -> Self {
        Self {
            id:       ListID::PunchPlay(value.id),
            name:     value.name,
            items:    value
                .parts
                .iter()
                .map(|x| ListItem {
                    id:       x.id,
                    added_at: x.release_date.and_time(Default::default()).and_utc(),
                })
                .collect(),
            readonly: true,
        }
    }
}

#[derive(Default, Clone, Copy, FromRepr, EnumCount, AsRefStr, EnumIter, EnumDiscriminants)]
#[strum_discriminants(vis())]
#[strum_discriminants(repr(usize))]
#[strum(serialize_all = "title_case")]
pub enum Sort {
    #[default]
    MostRecent,
    DateAdded,
    ReleaseDate,
    UserRating,
    Rating(RatingSource),
    Name,
    Relevance,
}
impl From<Sort> for usize {
    fn from(value: Sort) -> Self {
        SortDiscriminants::from(value) as usize
    }
}

#[allow(clippy::upper_case_acronyms)]
#[repr(usize)]
#[derive(Default, Clone, Copy, EnumIter, AsRefStr, FromRepr)]
pub enum RatingSource {
    #[default]
    IMDB,
    Letterboxd,
    Trakt,
    TMDB,
    Popcorn,
    Tomatoes,
}

#[derive(Serialize, Clone, Copy, Deserialize, Debug, Default)]
pub struct ExternalRatings {
    pub imdb:       (f64, u32),
    pub letterboxd: (f64, u32),
    pub trakt:      (u32, u32),
    pub tmdb:       (f64, u32),
    pub popcorn:    (u32, u32),
    pub tomatoes:   (u32, u32),
}

#[derive(Clone, EnumDiscriminants, Debug)]
#[strum_discriminants(derive(EnumIter, IntoStaticStr, EnumCount))]
#[strum_discriminants(repr(usize))]
pub enum FilterCriterion {
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
    Language(String, bool /*inverted*/),
    Country(String, bool /*inverted*/),
    Certification(Vec<String>, bool /*inverted*/),
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
    #[serde(alias = "watched_at")]
    pub date:   DateTime<Utc>,
    pub rating: f64,
    pub note:   Option<String>,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Entry {
    pub movie_id: u32,
    pub history:  Vec<HistoryEntry>,
}
impl From<PunchPlayHistoryItem> for Entry {
    fn from(value: PunchPlayHistoryItem) -> Self {
        Entry {
            movie_id: value.tmdb_id,
            history:  vec![HistoryEntry {
                date:   value.watched_at,
                rating: value.rating.unwrap() as f64,
                note:   None,
            }],
        }
    }
}
impl std::ops::Add for Entry {
    type Output = Entry;

    fn add(mut self, rhs: Self) -> Self::Output {
        for history_entry in rhs.history {
            self.history.push(history_entry);
        }
        self.history
            .sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());

        self.dedup_history();

        self
    }
}
impl Entry {
    pub fn get_user_rating(&self) -> f64 {
        self.history.last().map(|x| x.rating).unwrap_or(0.0)
    }

    pub fn get_latest_play(&self) -> DateTime<Utc> {
        self.history.last().map(|x| x.date).unwrap_or_default()
    }

    pub fn get_first_play(&self) -> DateTime<Utc> {
        self.history.first().map(|x| x.date).unwrap_or_default()
    }

    pub fn add_play(&mut self, date: DateTime<Utc>, rating: f64, note: Option<String>) {
        self.history.push(HistoryEntry { date, rating, note });
        self.history
            .sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());
    }

    pub fn dedup_history(&mut self) {
        if self.history.len() > 1 {
            let mut new_history = vec![];
            let mut i = 0;
            loop {
                if i >= self.history.len() - 1 {
                    if i == self.history.len() - 1 {
                        new_history.push(self.history.remove(i));
                    }

                    break;
                }

                let a = &self.history[i];
                let b = &self.history[i + 1];
                if b.date - a.date <= TimeDelta::days(1) {
                    new_history.push(HistoryEntry {
                        date:   a.date,
                        rating: a.rating.max(b.rating),
                        note:   None,
                    });
                    i += 1;
                } else {
                    new_history.push(HistoryEntry {
                        date:   a.date,
                        rating: a.rating,
                        note:   None,
                    });
                }

                i += 1;
            }

            // if new_history[0].date == DateTime::<Local>::default() {
            //     let x = if new_history.len() > 2 {if new_history.last().unwrap().date >= NaiveDate::from_ymd_opt(2025, 12, 1).unwrap().and_time(Default::default()).and_local_timezone(Local).latest().unwrap() {new_history.len() - 2} else {new_history.len() - 1}} else {1} - 1;
            //     let a = new_history.remove(0);
            //     let b = new_history.remove(x);

            //     new_history.insert(x, HistoryEntry { date: b.date, rating: a.rating.max(b.rating), note: None });
            // }

            self.history = new_history;
        }
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
impl From<&TMDBPerson> for Person {
    fn from(value: &TMDBPerson) -> Self {
        Self {
            id:     value.id,
            gender: value.gender,
            name:   value.name.clone(),
        }
    }
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Collection {
    pub id:    u32,
    pub name:  String,
    pub parts: Vec<u32>,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Credits {
    pub cast: Vec<Role>,
    pub crew: Vec<Role>,
}
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Movie {
    pub id:               u32,
    pub title:            String,
    pub release_date:     NaiveDate,
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
impl From<MovieDetails> for Movie {
    fn from(tmdb_details: MovieDetails) -> Self {
        info!("{tmdb_details:#?}");

        let (cast, crew) = if let Some(credits) = tmdb_details.credits.as_ref() {
            (
                credits
                    .cast
                    .iter()
                    .take(14)
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
        let released = tmdb_details.status == "Released";
        let external_ratings = if released {
            ExternalRatings {
                tmdb: (tmdb_details.vote_average, tmdb_details.vote_count),
                ..Default::default()
            }
        } else {
            Default::default()
        };
        Self {
            id: tmdb_details.id,
            title: tmdb_details.title.clone(),
            external_ratings,
            release_date: NaiveDate::parse_from_str(&tmdb_details.release_date, "%Y-%m-%d")
                .unwrap_or_default(),
            language: tmdb_details.original_language.clone(),
            genres: tmdb_details
                .genres
                .iter()
                .map(|x| x.name.to_string())
                .collect(),
            overview: tmdb_details.overview.clone(),
            tmdb_collection: tmdb_details.belongs_to_collection.clone().map(|x| x.id),
            runtime: tmdb_details.runtime,
            released: tmdb_details.status == "Released",
            tagline: tmdb_details.tagline.clone(),
            certification: tmdb_details
                .certificate
                .clone()
                .unwrap_or(if tmdb_details.adult { "N" } else { "NR" }.into()),
            origin_country: tmdb_details
                .origin_country
                .clone()
                .map(|x| x.first().unwrap_or(&"Unknown".into()).clone())
                .unwrap_or("Unknown".into()),
            credits: Credits { cast, crew },
            recommendations: tmdb_details.recommendations.clone().unwrap_or_default(),
        }
    }
}

impl Movie {
    pub fn add_trakt_details(&mut self, _trakt_details: TraktDetailsResponse) {
        info!("{_trakt_details:#?}");
    }

    pub fn add_punch_play_details(&mut self, punch_play_details: DetailsResponse) {
        info!("{punch_play_details:#?}");

        if self.released {
            if let Some(external_ratings) = punch_play_details.external_ratings {
                for external_rating in external_ratings.ratings {
                    if let Some(source) = external_rating.source.as_ref() {
                        if source == "imdb" {
                            self.external_ratings.imdb = (
                                external_rating.value.unwrap_or(0.0),
                                external_rating.votes.unwrap_or(0),
                            );
                        } else if source == "letterboxd" {
                            self.external_ratings.letterboxd = (
                                external_rating.value.unwrap_or(0.0),
                                external_rating.votes.unwrap_or(0),
                            );
                        } else if source == "trakt" {
                            self.external_ratings.trakt = (
                                external_rating.value.unwrap_or(0.0) as u32,
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

        if self.certification.is_empty() {
            if let Some(x) = punch_play_details.title.age_rating {
                self.certification = x;
            }
        }
        if self.recommendations.is_empty() {
            if let Some(x) = punch_play_details.title.recommendations {
                self.recommendations = x.into_iter().map(|x| x.tmdb_id).collect();
            }
        }
    }

    pub fn add_omdb_details(&mut self, omdb_details: OMDBDetailsResponse) {
        info!("{omdb_details:#?}");

        let rating = omdb_details.imdb_rating.parse::<f64>();
        let votes = omdb_details.imdb_votes.replace(',', "").parse::<u32>();
        if let (Ok(rating), Ok(votes)) = (rating, votes) {
            if votes > self.external_ratings.imdb.1 {
                self.external_ratings.imdb = (rating, votes);
            }
        }
    }

    pub fn get_external_rating(&self, source: RatingSource) -> Option<f64> {
        match source {
            RatingSource::IMDB =>
                (self.external_ratings.imdb.1 > 0).then_some(self.external_ratings.imdb.0),
            RatingSource::Letterboxd => (self.external_ratings.letterboxd.1 > 0)
                .then_some(self.external_ratings.letterboxd.0),
            RatingSource::Trakt =>
                (self.external_ratings.trakt.1 > 0).then_some(self.external_ratings.trakt.0 as f64),
            RatingSource::TMDB =>
                (self.external_ratings.tmdb.1 > 0).then_some(self.external_ratings.tmdb.0),
            RatingSource::Popcorn => (self.external_ratings.popcorn.1 > 0)
                .then_some(self.external_ratings.popcorn.0 as f64),
            RatingSource::Tomatoes => (self.external_ratings.tomatoes.1 > 0)
                .then_some(self.external_ratings.tomatoes.0 as f64),
        }
    }

    pub fn get_first_external_rating(&self) -> f64 {
        for source in RatingSource::iter() {
            let Some(rating) = self.get_external_rating(source) else {
                continue;
            };

            return rating;
        }

        f64::NAN
    }

    pub fn cmp_rating(&self, other: &Self, rating: RatingSource) -> Ordering {
        macro_rules! cmp_rating {
            ($field:ident) => {
                if self.external_ratings.$field.0 != other.external_ratings.$field.0 {
                    return self
                        .external_ratings
                        .$field
                        .0
                        .partial_cmp(&other.external_ratings.$field.0)
                        .unwrap_or(Ordering::Equal);
                } else {
                    return self
                        .external_ratings
                        .$field
                        .1
                        .partial_cmp(&other.external_ratings.$field.1)
                        .unwrap_or(Ordering::Equal);
                }
            };
        }

        match rating {
            RatingSource::IMDB => cmp_rating!(imdb),
            RatingSource::Letterboxd => cmp_rating!(letterboxd),
            RatingSource::Trakt => cmp_rating!(trakt),
            RatingSource::TMDB => cmp_rating!(tmdb),
            RatingSource::Popcorn => cmp_rating!(popcorn),
            RatingSource::Tomatoes => cmp_rating!(tomatoes),
        }
    }
}

impl std::cmp::PartialEq<Movie> for Movie {
    fn eq(&self, other: &Movie) -> bool {
        self.id == other.id
    }
}
