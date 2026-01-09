// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::bail;
use cargo_metadata::{semver::Version, FeatureName, Metadata, MetadataCommand, Node, PackageId};

const RING_VERSION: Version = Version::new(0, 17, 0);
const AWS_LC_RS_VERSION: Version = Version::new(1, 0, 0);

pub fn has_default_crypto_provider() -> anyhow::Result<()> {
    let metadata = metadata()?;
    let features = find_reqwest_features(&metadata)?;
    if !features.contains(&FeatureName::new("rustls-tls".to_string())) {
        bail!("reqwest should have rustls-tls enabled")
    }
    let features = find_rustls_features(&metadata)?;
    if !features.contains(&FeatureName::new("ring".to_string())) {
        bail!("rustls should have ring enabled")
    }
    let _id = find_resolved_dependency(&metadata, "ring", RING_VERSION)?;
    let id = find_resolved_dependency(&metadata, "aws-lc-rs", AWS_LC_RS_VERSION);
    if id.is_ok() {
        bail!("aws-lc-rs should not be a required dependency")
    }
    Ok(())
}

// TODO(#4170) - make this function verify that no crypto provided dependency
//   is linked.
pub fn no_default_crypto_provider() -> anyhow::Result<()> {
    let metadata = metadata()?;
    let features = find_reqwest_features(&metadata)?;
    if features.contains(&FeatureName::new("rustls-tls".to_string())) {
        bail!("reqwest should **not** have rustls-tls enabled")
    }
    let features = find_rustls_features(&metadata)?;
    if features.contains(&FeatureName::new("ring".to_string())) {
        bail!("rustls should **not** have ring enabled")
    }
    if let Ok(id) = find_resolved_dependency(&metadata, "ring", RING_VERSION) {
        bail!("ring should **not** be a resolved dependency: {id:?}")
    }
    if let Ok(id) = find_resolved_dependency(&metadata, "aws-lc-rs", RING_VERSION) {
        bail!("aws-lc-rs should **not** be a resolved dependency: {id:?}")
    }
    Ok(())
}

fn metadata() -> anyhow::Result<Metadata> {
    let metadata = MetadataCommand::new().exec()?;
    Ok(metadata)
}

fn find_reqwest_features(metadata: &Metadata) -> anyhow::Result<Vec<FeatureName>> {
    find_dependency_features(metadata, "reqwest", Version::new(0, 12, 0))
}

fn find_rustls_features(metadata: &Metadata) -> anyhow::Result<Vec<FeatureName>> {
    find_dependency_features(metadata, "rustls", Version::new(0, 23, 0))
}

fn find_dependency_id(
    metadata: &Metadata,
    name: &str,
    version: Version,
) -> anyhow::Result<PackageId> {
    let matches = metadata
        .packages
        .iter()
        .filter_map(|p| {
            if p.name == name && p.version >= version {
                Some(p.id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    match &matches[..] {
        [id] => Ok(id.clone()),
        [] => bail!("no matches for package {name}@{version:?}"),
        _ => bail!("too many matches for package {name}@{version:?}"),
    }
}

fn find_resolved_dependency(
    metadata: &Metadata,
    name: &str,
    version: Version,
) -> anyhow::Result<Node> {
    let id = find_dependency_id(metadata, name, version)?;
    let root = metadata
        .resolve
        .as_ref()
        .expect("metadata has resolved nodes");
    let Some(node) = root.nodes.iter().find(|n| n.id == id) else {
        bail!("could not find {name} in resolved dependencies")
    };
    Ok(node.clone())
}

fn find_dependency_features(
    metadata: &Metadata,
    name: &str,
    version: Version,
) -> anyhow::Result<Vec<FeatureName>> {
    let id = find_dependency_id(metadata, name, version)?;
    let root = metadata
        .resolve
        .as_ref()
        .expect("metadata has resolved nodes");
    let features = root
        .nodes
        .iter()
        .find(|n| n.id == id)
        .map(|n| n.features.clone())
        .unwrap_or_default();
    Ok(features)
}
