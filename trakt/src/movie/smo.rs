use serde::Deserialize;

#[derive(Deserialize, Debug, Default, Clone)]
pub(crate) struct SearchResponse {
    // score: String,
    // type: String,
    pub movie: SearchResponseMovie,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct SearchResponseMovie {
    pub title:                  String,
    pub year:                   Option<usize>,
    pub ids:                    SearchResponseID,
    pub tagline:                String,
    pub overview:               String,
    pub runtime:                usize,
    pub country:                String,
    pub trailer:                Option<String>,
    pub homepage:               String,
    pub status:                 String,
    pub rating:                 f64,
    pub votes:                  u32,
    pub comment_count:          String,
    pub updated_at:             String,
    pub language:               String,
    pub languages:              Vec<String>,
    pub available_translations: Vec<String>,
    pub genres:                 Vec<String>,
    pub subgenres:              Vec<String>,
    pub original_title:         String,
    pub released:               String,
    pub after_credits:          bool,
    pub during_credits:         bool,
    pub certification:          String,
    // images: MovieImages,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct SearchResponseID {
    pub trakt: u32,
    pub imdb:  String,
    pub tmdb:  u32,
    pub slug:  String,
    // plex: UNKNOWN
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct MovieDetails {
    pub title:                  String,
    pub year:                   Option<usize>,
    pub ids:                    IDs,
    pub tagline:                String,
    pub overview:               String,
    pub released:               String,
    pub runtime:                u32,
    pub country:                String,
    pub trailer:                Option<String>,
    pub homepage:               String,
    pub status:                 String,
    pub rating:                 f64,
    pub votes:                  u32,
    pub comment_count:          u32,
    pub updated_at:             String,
    pub language:               String,
    pub languages:              Vec<String>,
    pub available_translations: Vec<String>,
    pub genres:                 Vec<String>,
    pub certification:          Option<String>,
    pub(crate) images:          MovieImages,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub(crate) struct MovieImages {
    pub fanart: Vec<String>,
    pub poster: Vec<String>,
    pub banner: Vec<String>,
    // logo: Vec<String>,
    // clearart: Vec<String>,
    // thumb: Vec<String>,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct IDs {
    pub slug:  String,
    pub trakt: u32,
    pub imdb:  String,
    pub tmdb:  u32,
}
