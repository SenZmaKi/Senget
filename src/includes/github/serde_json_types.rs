//! Contains all Json types returned by various Github api calls

use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResponseJson {
    pub items: Vec<RepoResponseJson>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoResponseJson {
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub license: Option<License>,
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct License {
    pub name: Option<String>,
}

pub type ReleasesResponseJson = Vec<ReleaseResponseJson>;

pub type AssetsResponseJson = Vec<Asset>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseResponseJson {
    pub assets: AssetsResponseJson,
    pub tag_name: String,
    }


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub size: i64,
    pub browser_download_url: String,
}


