// Copyright 2024. The Tari Project
//
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
// disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
// following disclaimer in the documentation and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
// products derived from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
// INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
// USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
use anyhow::{anyhow, Error};
use log::{debug, error, info, warn};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use tari_common::configuration::Network;
use tauri_plugin_sentry::sentry;

use crate::{
    binaries::binaries_resolver::{VersionAsset, VersionDownloadInfo},
    download_utils::{extract, validate_checksum},
    github::request_client::RequestClient,
    progress_tracker_old::ProgressTracker,
};

use super::tapplets_resolver::LatestVersionApiAdapter;

pub const LOG_TARGET: &str = "tari::universe::tapplet_manager";

#[derive(Deserialize, Serialize, Default)]
pub struct TappletVersionsJsonContent {
    pub tapplets: HashMap<String, String>,
}
pub(crate) struct TappletManager {
    tapplet_name: String,
    tapplet_subfolder: Option<String>,
    version_requirements: VersionReq,
    network_prerelease_prefix: Option<String>,
    should_validate_checksum: bool,
    online_versions_list: Vec<VersionDownloadInfo>,
    local_aviailable_versions_list: Vec<Version>,
    used_version: Option<Version>,
    adapter: Box<dyn LatestVersionApiAdapter>,
}

impl TappletManager {
    pub fn new(
        tapplet_name: String,
        tapplet_subfolder: Option<String>,
        adapter: Box<dyn LatestVersionApiAdapter>,
        network_prerelease_prefix: Option<String>,
        should_validate_checksum: bool,
    ) -> Self {
        let versions_requirements_data = match Network::get_current_or_user_setting_or_default() {
            Network::NextNet => {
                include_str!("../../tapplets-versions/tapplets_versions_nextnet.json")
            }
            Network::Esmeralda => {
                include_str!("../../tapplets-versions/tapplets_versions_testnets.json")
            }
            Network::StageNet => {
                include_str!("../../tapplets-versions/tapplets_versions_mainnet.json")
            }
            Network::MainNet => {
                include_str!("../../tapplets-versions/tapplets_versions_mainnet.json")
            }
            Network::LocalNet => {
                include_str!("../../tapplets-versions/tapplets_versions_testnets.json")
            }
            Network::Igor => {
                include_str!("../../tapplets-versions/tapplets_versions_testnets.json")
            }
        };
        let version_requirements = TappletManager::read_version_requirements(
            tapplet_name.clone(),
            versions_requirements_data,
        );

        Self {
            tapplet_name: tapplet_name.clone(),
            tapplet_subfolder,
            should_validate_checksum,
            network_prerelease_prefix,
            version_requirements,
            online_versions_list: Vec::new(),
            local_aviailable_versions_list: Vec::new(),
            used_version: None,
            adapter,
        }
    }

    pub fn tapplet_subfolder(&self) -> Option<&String> {
        self.tapplet_subfolder.as_ref()
    }

    fn read_version_requirements(tapplet_name: String, data_str: &str) -> VersionReq {
        let json_content: TappletVersionsJsonContent =
            serde_json::from_str(data_str).unwrap_or_default();
        let version_requirement = json_content.tapplets.get(&tapplet_name)
            .and_then(|version_req| VersionReq::from_str(version_req).ok())
            .unwrap_or_else(|| {
                error!(target: LOG_TARGET, "Error parsing version requirements for tapplet: {:?}", tapplet_name);
                debug!(target: LOG_TARGET, "App will try to run with highest version found");
                VersionReq::default()
            });

        debug!(target: LOG_TARGET, "Version requirements for {:?}: {:?}", tapplet_name, version_requirement);

        version_requirement
    }

    fn select_highest_local_version(&mut self) -> Option<Version> {
        info!(target: LOG_TARGET,"Selecting highest local version for tapplet: {:?}", self.tapplet_name);

        if self.local_aviailable_versions_list.is_empty() {
            warn!(target: LOG_TARGET,"No local versions found for tapplet: {:?}", self.tapplet_name);
            return None;
        }

        let selected_local_version = Some(self.local_aviailable_versions_list[0].clone());

        debug!(target: LOG_TARGET,"Selected local version: {:?}", selected_local_version);
        selected_local_version.clone()
    }

    fn select_highest_online_version(&mut self) -> Option<Version> {
        debug!(target: LOG_TARGET,"Selecting highest online version for tapplet: {:?}", self.tapplet_name);

        if self.online_versions_list.is_empty() {
            warn!(target: LOG_TARGET,"No online versions found for tapplet: {:?}", self.tapplet_name);
            return None;
        }

        let selected_online_version = Some(self.online_versions_list[0].version.clone());

        debug!(target: LOG_TARGET,"Selected online version: {:?}", selected_online_version);
        selected_online_version.clone()
    }

    fn create_in_progress_folder_for_selected_version(
        &self,
        selected_version: Version,
    ) -> Result<PathBuf, Error> {
        debug!(target: LOG_TARGET,"Creating in progress folder for version: {:?}", selected_version);

        let tapplet_folder = self.adapter.get_tapplet_folder().map_err(|error| {
            error!(target: LOG_TARGET, "Error getting tapplet folder. Error: {:?}", error);
            anyhow!("Error getting tapplet folder: {:?}", error)
        })?;

        let in_progress_folder = tapplet_folder
            .join(selected_version.to_string())
            .join("in_progress");

        if in_progress_folder.exists() {
            debug!(target: LOG_TARGET,"Removing in progress folder: {:?}", in_progress_folder);
            if let Err(error) = std::fs::remove_dir_all(&in_progress_folder) {
                error!(target: LOG_TARGET, "Error removing in progress folder: {:?}. Error: {:?}", in_progress_folder, error);
            }
        }

        debug!(target: LOG_TARGET,"Creating in progress folder: {:?}", in_progress_folder);
        std::fs::create_dir_all(&in_progress_folder)?;

        Ok(in_progress_folder)
    }

    async fn delete_in_progress_folder_for_selected_version(
        &self,
        selected_version: Version,
        progress_tracker: ProgressTracker,
    ) -> Result<(), Error> {
        debug!(target: LOG_TARGET,"Deleting in progress folder for version: {:?}", selected_version);

        let tapplet_folder = self.adapter.get_tapplet_folder().map_err(|error| {
            error!(target: LOG_TARGET, "Error getting tapplet folder. Error: {:?}", error);
            anyhow!("Error getting tapplet folder: {:?}", error)
        })?;

        let in_progress_folder = tapplet_folder
            .join(selected_version.to_string())
            .join("in_progress");

        progress_tracker
            .send_last_action(format!(
                "Removing in progress folder: {:?}",
                in_progress_folder
            ))
            .await;
        if in_progress_folder.exists() {
            debug!(target: LOG_TARGET,"Removing in progress folder: {:?}", in_progress_folder);
            if let Err(error) = std::fs::remove_dir_all(&in_progress_folder) {
                error!(target: LOG_TARGET, "Error removing in progress folder: {:?}. Error: {:?}", in_progress_folder, error);
            }
        }

        Ok(())
    }

    fn get_asset_for_selected_version(
        &self,
        selected_version: Version,
    ) -> Result<VersionAsset, Error> {
        debug!(target: LOG_TARGET,"Getting asset for selected version: {:?}", selected_version);

        let version_info = self
            .online_versions_list
            .iter()
            .find(|v| v.version.eq(&selected_version))
            .ok_or_else(|| anyhow!("No version info found for version: {:?}", selected_version))?;

        debug!(target: LOG_TARGET, "Found version info for version: {:?}", selected_version);

        self.adapter
            .find_version_for_platform(version_info)
            .inspect(|_asset| {
                debug!(target: LOG_TARGET, "Found asset for version: {:?}", selected_version);
            })
            .map_err(|error| {
                anyhow!(
                    "Error finding asset for version: {:?}. Error: {:?}",
                    selected_version,
                    error
                )
            })
    }

    fn ensure_empty_directory(&self, dir: PathBuf) -> Result<(), Error> {
        if dir.exists() {
            warn!(target: LOG_TARGET, "Destination dir exists. Removing all files from: {:?}", dir.clone());
            std::fs::remove_dir_all(dir.clone())
                .and_then(|_| std::fs::create_dir_all(dir.clone()))
                .map_err(|e| {
                    anyhow!(
                        "Error handling destination dir: {:?}. Error: {:?}",
                        dir.clone(),
                        e
                    )
                })
        } else {
            std::fs::create_dir_all(dir.clone()).map_err(|e| {
                anyhow!(
                    "Error creating destination dir: {:?}. Error: {:?}",
                    dir.clone(),
                    e
                )
            })
        }
    }

    async fn validate_checksum(
        &self,
        version: &Version,
        asset: VersionAsset,
        destination_dir: PathBuf,
        in_progress_file_zip: PathBuf,
        progress_tracker: ProgressTracker,
    ) -> Result<(), Error> {
        info!(target: LOG_TARGET, "Validating checksum for tapplet: {} with version: {:?}", self.tapplet_name, version);
        let version_download_info = VersionDownloadInfo {
            version: version.clone(),
            assets: vec![asset.clone()],
        };
        progress_tracker
            .send_last_action(format!(
                "Downloading checksum file for dest: {:?}",
                destination_dir
            ))
            .await;
        let checksum_file = self
            .adapter
            .download_and_get_checksum_path(
                destination_dir.clone().to_path_buf(),
                version_download_info,
            )
            .await
            .map_err(|e| {
                std::fs::remove_dir_all(destination_dir.clone()).ok();
                anyhow!(
                    "Error downloading checksum file for version: {:?}. Error: {:?}",
                    version,
                    e
                )
            })?;

        let expected_checksum = self
            .adapter
            .get_expected_checksum(checksum_file.clone(), &asset.name)
            .await?;

        progress_tracker
            .send_last_action(format!(
                "Validating checksum for checksum file: {:?} and in progress file: {:?}",
                checksum_file, in_progress_file_zip
            ))
            .await;
        match validate_checksum(in_progress_file_zip.clone(), expected_checksum).await {
            Ok(validate_checksum) => {
                if validate_checksum {
                    info!(target: LOG_TARGET, "Checksum validation succeeded for tapplet: {} with version: {:?}", self.tapplet_name, version);
                    Ok(())
                } else {
                    std::fs::remove_dir_all(destination_dir.clone()).ok();
                    Err(anyhow!("Checksums mismatched!"))
                }
            }
            Err(e) => {
                std::fs::remove_dir_all(destination_dir.clone()).ok();
                Err(anyhow!(
                    "Checksum validation failed for version: {:?}. Error: {:?}",
                    version,
                    e
                ))
            }
        }
    }

    fn check_if_version_meet_requirements(&self, version: &Version) -> bool {
        info!(target: LOG_TARGET,"Checking if version meets requirements: {:?}", version);
        info!(target: LOG_TARGET,"Version requirements: {:?}", self.version_requirements);
        let is_meet_semver = self.version_requirements.matches(version);
        let did_meet_network_prerelease = self
            .network_prerelease_prefix
            .as_ref()
            .is_none_or(|prefix| version.pre.matches(prefix).any(|_| true));

        debug!(target: LOG_TARGET,"Version meets semver requirements: {:?}", is_meet_semver);
        debug!(target: LOG_TARGET,"Version meets network prerelease requirements: {:?}", did_meet_network_prerelease);

        is_meet_semver && did_meet_network_prerelease
    }

    fn check_if_version_exceeds_requirements(&self, version: &Version) -> bool {
        !self
            .online_versions_list
            .iter()
            .any(|v| v.version.gt(version))
    }

    pub fn select_highest_version(&mut self) -> Option<Version> {
        debug!(target: LOG_TARGET,"Selecting version for tapplet: {:?}", self.tapplet_name);

        let online_selected_version = self.select_highest_online_version();
        let local_selected_version = self.select_highest_local_version();

        debug!(target: LOG_TARGET,"Online selected version: {:?}", online_selected_version);
        debug!(target: LOG_TARGET,"Local selected version: {:?}", local_selected_version);

        let highest_version = Version::max(
            online_selected_version.unwrap_or(Version::new(0, 0, 0)),
            local_selected_version.unwrap_or(Version::new(0, 0, 0)),
        );

        if highest_version == Version::new(0, 0, 0) {
            warn!(target: LOG_TARGET,"No highest version selected for {:?} tapplet", self.tapplet_name);
            return None;
        }

        debug!(target: LOG_TARGET,"Selected highest version: {:?}", highest_version);

        Some(highest_version.clone())
    }

    pub fn check_if_files_for_version_exist(&self, version: Option<Version>) -> bool {
        debug!(target: LOG_TARGET,"Checking if files for selected version exist: {:?}", version);
        info!(target: LOG_TARGET,"Checking if files for selected version exist: {:?}", version);

        if let Some(version) = version {
            info!(target: LOG_TARGET, "Selected version: {:?}", version);

            let tapplet_folder = match self.adapter.get_tapplet_folder() {
                Ok(path) => path,
                Err(e) => {
                    error!(target: LOG_TARGET, "Error getting tapplet folder. Error: {:?}", e);
                    return false;
                }
            };

            info!(target: LOG_TARGET, "Tapplet folder path: {:?}", tapplet_folder);
            let version_folder = tapplet_folder.join(version.to_string());

            // difference between binaries process: for a tapplet just check if index.html exists
            let tapplet_file_with_html = version_folder.join("index.html");

            info!(target: LOG_TARGET, "Version folder path: {:?}", version_folder);
            info!(target: LOG_TARGET, "Tapplet file path with html: {:?}", tapplet_file_with_html);

            let tapplet_file_exists = tapplet_file_with_html.exists();

            info!(target: LOG_TARGET, "tapplet file exists: {:?}", tapplet_file_exists);

            return tapplet_file_exists;
        }
        warn!(target: LOG_TARGET, "No version selected");
        false
    }

    pub async fn check_for_updates(&mut self) {
        debug!(target: LOG_TARGET,"Checking for updates for tapplet: {:?}", self.tapplet_name);

        let versions_info = self.adapter.fetch_releases_list().await.unwrap_or_default();

        debug!(target: LOG_TARGET,
            "Found {:?} versions for tapplet: {:?}",
            versions_info.len(),
            self.tapplet_name
        );

        for version_info in versions_info {
            if self.check_if_version_meet_requirements(&version_info.version) {
                debug!(target: LOG_TARGET,"Adding version to online versions list: {:?}", version_info.version);
                self.online_versions_list.push(version_info);
            } else {
                debug!(target: LOG_TARGET,"Skipping version: {:?}", version_info.version);
                if self.check_if_version_exceeds_requirements(&version_info.version) {
                    warn!(target: LOG_TARGET,"Version: {:?} is higher then maximum version from requirements", version_info.version);
                }
            }
        }

        self.online_versions_list
            .sort_by(|a, b| a.version.cmp(&b.version));
        self.online_versions_list.reverse();
    }

    pub async fn download_version_with_retries(
        &self,
        selected_version: Option<Version>,
        progress_tracker: ProgressTracker,
    ) -> Result<(), Error> {
        let mut last_error_message = String::new();
        for retry in 0..3 {
            match self
                .download_selected_version(selected_version.clone(), progress_tracker.clone())
                .await
            {
                Ok(_) => return Ok(()),
                Err(error) => {
                    last_error_message = format!(
                        "Failed to download tapplet: {}. Error: {:?}",
                        self.tapplet_name, error
                    );
                    warn!(target: LOG_TARGET, "Failed to download tapplet: {} at retry: {}", self.tapplet_name, retry);
                    continue;
                }
            }
        }
        sentry::capture_message(&last_error_message, sentry::Level::Error);
        error!(target: LOG_TARGET, "{}", last_error_message);
        Err(anyhow!(last_error_message))
    }

    #[allow(clippy::too_many_lines)]
    async fn download_selected_version(
        &self,
        selected_version: Option<Version>,
        progress_tracker: ProgressTracker,
    ) -> Result<(), Error> {
        debug!(target: LOG_TARGET,"Downloading version: {:?}", selected_version);

        let version = match selected_version {
            Some(version) => version,
            None => {
                warn!(target: LOG_TARGET, "Download {:?} tapplet version: no version selected", self.tapplet_name);
                return Err(anyhow!(format!(
                    "Download {:?} tapplet version: no version selected",
                    self.tapplet_name
                )));
            }
        };

        let asset = self
            .get_asset_for_selected_version(version.clone())
            .map_err(|e| {
                anyhow!(
                    "Error getting asset for version: {:?}. Error: {:?}",
                    version,
                    e
                )
            })?;

        let tapplet_folder = self
            .adapter
            .get_tapplet_folder()
            .map_err(|e| anyhow!("Error getting tapplet folder: {:?}", e))?;

        let destination_dir = tapplet_folder.join(version.to_string());

        // This is a safety check to ensure that the destination directory is empty
        // Its special case for tari repo, where zip will inclue mutliple tapplets
        // So when one of them is deleted, and we need to download it again
        // We in fact will download zip with multiple tapplets, and when other tapplets are present in destination dir
        // extract will fail, so we need to remove all files from destination dir
        self.ensure_empty_directory(destination_dir.clone())?;

        let in_progress_dir = self
            .create_in_progress_folder_for_selected_version(version.clone())
            .map_err(|e| anyhow!("Error creating in progress folder. Error: {:?}", e))?;
        let in_progress_file_zip = in_progress_dir.join(asset.name.clone());

        let download_url = asset.clone().url;
        let fallback_url = asset.clone().fallback_url;

        info!(target: LOG_TARGET, "Downloading tapplet: {} from url: {}", self.tapplet_name, download_url);
        progress_tracker
            .send_last_action(format!(
                "Downloading tapplet: {} with version: {}",
                self.tapplet_name, version
            ))
            .await;

        if RequestClient::current()
            .download_file(
                download_url.as_str(),
                &in_progress_file_zip,
                asset.source.is_mirror(),
            )
            .await
            .map_err(|e| anyhow!("Error downloading version: {:?}. Error: {:?}", version, e))
            .is_err()
        {
            if let Some(fallback_url) = fallback_url {
                info!(target: LOG_TARGET, "Downloading tapplet: {} from fallback url: {}", self.tapplet_name, fallback_url);
                progress_tracker
                    .send_last_action(format!(
                        "Downloading tapplet: {} with version: {} from fallback url",
                        self.tapplet_name, version
                    ))
                    .await;

                RequestClient::current()
                    .download_file(
                        fallback_url.as_str(),
                        &in_progress_file_zip,
                        asset.source.is_mirror(),
                    )
                    .await
                    .map_err(|e| {
                        anyhow!("Error downloading version: {:?}. Error: {:?}", version, e)
                    })?;
            } else {
                return Err(anyhow!(
                    "Error downloading version: {:?}. No fallback url provided",
                    version
                ));
            }
        }

        progress_tracker
            .send_last_action(format!(
                "Extracting file: {} to dest: {}",
                in_progress_file_zip.to_str().unwrap_or_default(),
                destination_dir.to_str().unwrap_or_default()
            ))
            .await;
        extract(&in_progress_file_zip, &destination_dir)
            .await
            .map_err(|e| anyhow!("Error extracting version: {:?}. Error: {:?}", version, e))?;

        if self.should_validate_checksum {
            self.validate_checksum(
                &version,
                asset,
                destination_dir,
                in_progress_file_zip,
                progress_tracker.clone(),
            )
            .await?;
        }

        self.delete_in_progress_folder_for_selected_version(
            version.clone(),
            progress_tracker.clone(),
        )
        .await?;
        Ok(())
    }

    pub async fn read_local_versions(&mut self) {
        debug!(target: LOG_TARGET,"Reading local versions for tapplet: {:?}", self.tapplet_name);

        let tapplet_folder = match self.adapter.get_tapplet_folder() {
            Ok(path) => path,
            Err(e) => {
                error!(target: LOG_TARGET,"Error getting tapplet folder. Error: {:?}", e);
                return;
            }
        };

        let version_folders_list = match std::fs::read_dir(tapplet_folder) {
            Ok(list) => list,
            Err(e) => {
                error!(target: LOG_TARGET, "Error reading tapplet folder. Error: {:?}", e);
                return;
            }
        };

        version_folders_list.filter_map(Result::ok).for_each(|version_folder| {
            if let Ok(file_type) = version_folder.file_type() {
                if file_type.is_dir() {
                    if let Some(version_folder_name) = version_folder.file_name().to_str() {
                        match Version::from_str(version_folder_name) {
                            Ok(version) => {
                                debug!(target: LOG_TARGET, "Found local version: {:?}", version);
                                if self.check_if_version_meet_requirements(&version)
                                    && self.check_if_files_for_version_exist(Some(version.clone()))
                                {
                                    debug!(target: LOG_TARGET, "Adding local version to list: {:?}", version);
                                    self.local_aviailable_versions_list.push(version);
                                }
                            }
                            Err(e) => {
                                error!("Error parsing version folder name: {:?}", e);
                            }
                        }
                    } else {
                        error!(target: LOG_TARGET, "Error getting version folder name");
                    }
                }
            } else {
                error!(target: LOG_TARGET, "Error getting file type. Error");
            }
        });
    }

    pub fn set_used_version(&mut self, version: Version) {
        debug!(target: LOG_TARGET,"Setting used version: {:?}", version);
        self.used_version = Some(version);
    }

    pub fn get_used_version(&self) -> Option<Version> {
        self.used_version.clone()
    }

    pub fn get_base_dir(&self) -> Result<PathBuf, Error> {
        self.adapter
            .get_tapplet_folder()
            .and_then(|path| {
                self.used_version
                    .clone()
                    .map(|version| path.join(version.to_string()))
                    .ok_or_else(|| anyhow!("No version selected"))
            })
            .map_err(|e| anyhow!("Error getting tapplet folder. Error: {:?}", e))
    }
}
