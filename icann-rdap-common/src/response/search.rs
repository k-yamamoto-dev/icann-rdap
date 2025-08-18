//! RDAP Search Results.
use {
    crate::prelude::{Common, Extension},
    serde::{Deserialize, Serialize},
};

use super::{domain::Domain, entity::Entity, nameserver::Nameserver, CommonFields, ToResponse};

/// Represents RDAP domain search results.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Eq)]
pub struct DomainSearchResults {
    #[serde(flatten)]
    pub common: Common,

    #[serde(rename = "domainSearchResults")]
    pub results: Vec<Domain>,
}

#[buildstructor::buildstructor]
impl DomainSearchResults {
    /// Builds a domain search result.
    #[builder(visibility = "pub")]
    fn new(results: Vec<Domain>, extensions: Vec<Extension>) -> Self {
        Self {
            common: Common::level0().extensions(extensions).build(),
            results,
        }
    }

    /// Get the domains in the search.
    pub fn results(&self) -> &[Domain] {
        self.results.as_ref()
    }
}

impl CommonFields for DomainSearchResults {
    fn common(&self) -> &Common {
        &self.common
    }
}

impl ToResponse for DomainSearchResults {
    fn to_response(self) -> super::RdapResponse {
        super::RdapResponse::DomainSearchResults(Box::new(self))
    }
}

/// Represents RDAP nameserver search results.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Eq)]
pub struct NameserverSearchResults {
    #[serde(flatten)]
    pub common: Common,

    #[serde(rename = "nameserverSearchResults")]
    pub results: Vec<Nameserver>,
}

#[buildstructor::buildstructor]
impl NameserverSearchResults {
    /// Builds a nameserver search result.
    #[builder(visibility = "pub")]
    fn new(results: Vec<Nameserver>, extensions: Vec<Extension>) -> Self {
        Self {
            common: Common::level0().extensions(extensions).build(),
            results,
        }
    }

    /// Get the nameservers in the search.
    pub fn results(&self) -> &[Nameserver] {
        self.results.as_ref()
    }
}

impl CommonFields for NameserverSearchResults {
    fn common(&self) -> &Common {
        &self.common
    }
}

impl ToResponse for NameserverSearchResults {
    fn to_response(self) -> super::RdapResponse {
        super::RdapResponse::NameserverSearchResults(Box::new(self))
    }
}

/// Represents RDAP entity search results.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Eq)]
pub struct EntitySearchResults {
    #[serde(flatten)]
    pub common: Common,

    #[serde(rename = "entitySearchResults")]
    pub results: Vec<Entity>,
}

#[buildstructor::buildstructor]
impl EntitySearchResults {
    /// Builds an entity search result.
    #[builder(visibility = "pub")]
    fn new(results: Vec<Entity>, extensions: Vec<Extension>) -> Self {
        Self {
            common: Common::level0().extensions(extensions).build(),
            results,
        }
    }

    /// Get the entities in the search.
    pub fn results(&self) -> &[Entity] {
        self.results.as_ref()
    }
}

impl CommonFields for EntitySearchResults {
    fn common(&self) -> &Common {
        &self.common
    }
}

impl ToResponse for EntitySearchResults {
    fn to_response(self) -> super::RdapResponse {
        super::RdapResponse::EntitySearchResults(Box::new(self))
    }
}
