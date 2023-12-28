//! Interacts with the github api

use crate::{
    github::serde_json_types::{
        AssetsResponseJson, ReleasesResponseJson, RepoResponseJson, SearchResponseJson,
    },
    includes::dist::{Dist, DistType, PackageInfo},
};
use core::fmt;
use regex::{self, Regex};
use serde::{Deserialize, Serialize};

use super::serde_json_types::{Asset, ReleaseResponseJson};

const GITHUB_API_ENTRY_POINT: &str = "https://api.github.com";

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    pub name: String,
    pub full_name: String, // For example if the url is https://github.com/SenZmaKi/Senpwai the full_name is SenZmaKi/Senpwai
    pub url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub license: Option<String>,
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = &self.name;
        let url = &self.url;
        let language = self.language.clone().unwrap_or_default();
        let description = self.description.clone().unwrap_or_default();
        let author = self.full_name.split('/').collect::<Vec<&str>>()[0];
        let license = self.license.clone().unwrap_or_default();

        write!(
            f,
            "Name: {}\nAuthor: {}\nDescription: {}\nRepository Url: {}\nPrimary Language: {}\nLicense: {}",
            name, author, description, url, language, license
        )
    }
}
struct AssetInfo {
    pub file_title: String,
    pub download_url: String,
    pub dist_type: DistType,
}

impl Repo {
    pub fn new(
        name: String,
        full_name: String,
        url: String,
        description: Option<String>,
        language: Option<String>,
        license: Option<String>,
    ) -> Repo {
        Repo {
            url,
            name,
            full_name,
            description,
            language,
            license,
        }
    }

    pub fn generate_version_regex() -> Regex {
        Regex::new(r"(\d+(\.\d+)*)").unwrap()
    }

    async fn get_assets_url_by_version(
        &self,
        version: &str,
        client: &reqwest::Client,
        version_regex: &Regex,
    ) -> Result<Option<(String, String)>, reqwest::Error> {
        let url = self.generate_endpoint("releases");
        let releases_response_json: ReleasesResponseJson =
            client.get(url).send().await?.json().await?;
        if releases_response_json.is_empty() {
            Ok(None)
        } else {
            let version = match Repo::parse_version(version, version_regex) {
                None => return Ok(None),
                Some(v) => v,
            };
            for r in releases_response_json {
                let curr_ver = match Repo::parse_version(&r.tag_name, version_regex) {
                    None => continue,
                    Some(v) => v,
                };
                if version == curr_ver {
                    return Ok(Some((r.assets_url, curr_ver.to_owned())));
                }
            }
            Ok(None)
        }
    }

    fn parse_asset_info(repo_name_lower: &str, asset: &Asset) -> Option<AssetInfo> {
        let name_lower = asset.name.to_lowercase();
        if !name_lower.contains(repo_name_lower) {
            return None;
        }
        let inner_file_extension = name_lower.split('.').last().unwrap_or_default();
        let is_zip_dist = inner_file_extension == "zip" && name_lower.contains("win");
        let is_exe = inner_file_extension == "exe";
        let is_installer_dist = inner_file_extension == "msi"
            || (is_exe && (name_lower.contains("installer") || name_lower.contains("setup")));
        let is_exe_dist = !is_installer_dist && is_exe;
        if is_exe_dist || is_zip_dist || is_installer_dist {
            let dist_type = if is_exe_dist {
                DistType::Exe
            } else if is_zip_dist {
                DistType::Zip
            } else {
                DistType::Installer
            };
            return Some(AssetInfo {
                file_title: asset.name.clone(),
                download_url: asset.browser_download_url.clone(),
                dist_type,
            });
        }
        None
    }

    fn find_preferred_dist(
        preferred_dist_type: &Option<DistType>,
        asset_infos: Vec<AssetInfo>,
        repo_name: String,
        version: String,
    ) -> Option<Dist> {
        match preferred_dist_type {
            None => {
                // The order in which each installer type is compared is: Exe > Zip > Normal
                let asset_info = asset_infos.into_iter()
                .max_by_key(|ai| ai.dist_type.clone()).unwrap();
                let dist = PackageInfo::new(
                    repo_name,
                    asset_info.download_url,
                    version,
                    asset_info.file_title,
                )
                .fetch_dist(asset_info.dist_type);
                Some(dist)
            }

            Some(pref_inst) => {
                let dist = asset_infos
                    .iter()
                    .find(|ai| ai.dist_type == *pref_inst)
                    .map(|ai| {
                        let pi = PackageInfo::new(
                            repo_name,
                            ai.download_url.clone(),
                            version,
                            ai.file_title.clone(),
                        );
                        pi.fetch_dist(ai.dist_type.clone())
                    });
                dist
            }
        }
    }

    fn parse_assets_for_distributable(
        &self,
        assets: AssetsResponseJson,
        version: String,
        preferred_dist_type: &Option<DistType>,
    ) -> Option<Dist> {
        let repo_name_lower = self.name.to_lowercase();
        let asset_infos: Vec<AssetInfo> = assets
            .iter()
            .filter_map(|asset| Repo::parse_asset_info(&repo_name_lower, asset))
            .collect();
        if asset_infos.is_empty() {
            return None;
        };
        Repo::find_preferred_dist(
            preferred_dist_type,
            asset_infos,
            self.name.clone(),
            version,
        )
    }

    pub async fn get_dist(
        &self,
        client: &reqwest::Client,
        version: &str,
        version_regex: &Regex,
        preferred_dist_type: &Option<DistType>,
    ) -> Result<Option<Dist>, reqwest::Error> {
        let (target_assets_url, parsed_version) = match self
            .get_assets_url_by_version(version, client, version_regex)
            .await?
        {
            None => return Ok(None),
            Some(asset_url_and_version) => asset_url_and_version,
        };
        let assets = client.get(target_assets_url).send().await?.json().await?;
        Ok(self.parse_assets_for_distributable(assets, parsed_version, preferred_dist_type))
    }
    pub async fn get_latest_dist(
        &self,
        client: &reqwest::Client,
        version_regex: &Regex,
        preferred_dist_type: &Option<DistType>,
    ) -> Result<Option<Dist>, reqwest::Error> {
        let url = self.generate_endpoint("releases/latest");
        let response = client.get(url).send().await?;
        if response.status() == 404 {
            return Ok(None);
        }
        let release_response_json: ReleaseResponseJson = response.json().await?;
        if let Some(version) = Repo::parse_version(&release_response_json.tag_name, version_regex) {
            return Ok(self.parse_assets_for_distributable(
                release_response_json.assets,
                version.to_owned(),
                preferred_dist_type,
            ));
        }
        Ok(None)
    }
    fn generate_endpoint(&self, resource: &str) -> String {
        format!(
            "{}/repos/{}/{}",
            GITHUB_API_ENTRY_POINT, self.full_name, resource
        )
    }
    pub fn parse_version<'a>(text: &'a str, version_regex: &Regex) -> Option<&'a str> {
        let mat: regex::Match = version_regex.find(text)?;
        Some(mat.as_str())
    }
}

fn extract_repo(repo_response_json: RepoResponseJson) -> Repo {
    Repo::new(
        repo_response_json.name,
        repo_response_json.full_name,
        repo_response_json.html_url,
        repo_response_json.description,
        repo_response_json.language,
        repo_response_json.license.and_then(|l| l.name),
    )
}
pub async fn search(query: &str, client: &reqwest::Client) -> Result<Vec<Repo>, reqwest::Error> {
    let url = format!("{GITHUB_API_ENTRY_POINT}/search/repositories?q={query}&per_page=10");
    let search_response_json: SearchResponseJson = client.get(url).send().await?.json().await?;
    let results = search_response_json
        .items
        .into_iter()
        .map(extract_repo)
        .collect();
    Ok(results)
}

#[cfg(test)]
pub mod tests {

    use crate::includes::{
        dist::{self, Dist},
        github::api::{search, Repo},
        test_utils::{client, hatt_repo, senpwai_repo},
    };

    #[tokio::test]
    async fn test_search() {
        let queries = vec!["Senpwai", "empty-repo", "zohofberibp09u0&_+*"];
        for (idx, query) in queries.iter().enumerate() {
            println!("\nResults of Search {}\n", idx + 1);
            let search_results = search(query, &client()).await.expect("Ok(search_results)");
            for r in search_results.iter() {
                println!("{}\n", r)
            }
        }
    }

    #[tokio::test]
    async fn test_getting_latest_distributable() {
        let dist = senpwai_repo()
            .get_latest_dist(&client(), &Repo::generate_version_regex(), &None)
            .await
            .expect("Ok(dist)");
        let dist = dist.expect("Some(dist)");
        println!("\nResults getting latest dist\n");
        println!("{:?}\n", dist);
    }

    #[tokio::test]
    async fn test_getting_installer_dist() {
        let dist = hatt_repo()
            .get_dist(
                &client(),
                "0.3.5",
                &Repo::generate_version_regex(),
                &Some(dist::DistType::Installer),
            )
            .await
            .expect("Ok(dist)")
            .expect("Some(dist)");
        let dist = match dist {
            Dist::Installer(dist) => dist,
            _ => panic!("Distributable is not installer"),
        };
        assert_eq!(dist.package_info.version, "0.3.5");
    }

    async fn test_getting_exe_distributable() {
        let dist = hatt_repo()
            .get_dist(
                &client(),
                "0.3.1",
                &Repo::generate_version_regex(),
                &Some(dist::DistType::Exe),
            )
            .await
            .expect("Ok(dist)")
            .expect("Some(dist)");
        let dist = match dist {
            Dist::Exe(dist) => dist,
            _ => panic!("Distributable is not exe"),
        };
        assert_eq!(dist.package_info.version, "0.3.1");
    }
}
