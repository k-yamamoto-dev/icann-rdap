//! The IANA RDAP Bootstrap Registries.

use {
    ipnet::{Ipv4Net, Ipv6Net},
    prefix_trie::PrefixMap,
    serde::{Deserialize, Serialize},
    thiserror::Error,
};

/// IANA registry variants for RDAP.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IanaRegistryType {
    RdapBootstrapDns,
    RdapBootstrapAsn,
    RdapBootstrapIpv4,
    RdapBootstrapIpv6,
    RdapObjectTags,
}

impl IanaRegistryType {
    /// Get the URL for an IANA RDAP registry.
    pub fn url(&self) -> &str {
        match self {
            Self::RdapBootstrapDns => "https://data.iana.org/rdap/dns.json",
            Self::RdapBootstrapAsn => "https://data.iana.org/rdap/asn.json",
            Self::RdapBootstrapIpv4 => "https://data.iana.org/rdap/ipv4.json",
            Self::RdapBootstrapIpv6 => "https://data.iana.org/rdap/ipv6.json",
            Self::RdapObjectTags => "https://data.iana.org/rdap/object-tags.json",
        }
    }

    /// Get the filename in the URL for the IANA RDAP registry.
    pub fn file_name(&self) -> &str {
        let url = self.url();
        url.rsplit('/')
            .next()
            .expect("unexpected errror: cannot get filename from url")
    }
}

/// Classes of IANA registries.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum IanaRegistry {
    RdapBootstrapRegistry(RdapBootstrapRegistry),
    // might add IANA registrar IDs later
}

/// Represents an IANA RDAP bootstrap registry.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RdapBootstrapRegistry {
    pub version: String,
    pub publication: String,
    pub description: Option<String>,
    pub services: Vec<Vec<Vec<String>>>,
}

pub trait BootstrapRegistry {
    fn get_dns_bootstrap_urls(&self, ldh: &str) -> Result<Vec<String>, BootstrapRegistryError>;
    fn get_asn_bootstrap_urls(&self, asn: &str) -> Result<Vec<String>, BootstrapRegistryError>;
    fn get_ipv4_bootstrap_urls(&self, ipv4: &str) -> Result<Vec<String>, BootstrapRegistryError>;
    fn get_ipv6_bootstrap_urls(&self, ipv6: &str) -> Result<Vec<String>, BootstrapRegistryError>;
    fn get_tag_bootstrap_urls(&self, tag: &str) -> Result<Vec<String>, BootstrapRegistryError>;
}

/// Errors from processing IANA RDAP bootstrap registries.
#[derive(Debug, Error)]
pub enum BootstrapRegistryError {
    #[error("Empty Service")]
    EmptyService,
    #[error("Empty URL Set")]
    EmptyUrlSet,
    #[error("Invalid Bootstrap Input")]
    InvalidBootstrapInput,
    #[error("No Bootstrap URLs Found")]
    NoBootstrapUrls,
    #[error("Invalid Bootstrap Service")]
    InvalidBootstrapService,
}

impl BootstrapRegistry for IanaRegistry {
    /// Get the URLs from the IANA domain bootstrap registry.
    fn get_dns_bootstrap_urls(&self, ldh: &str) -> Result<Vec<String>, BootstrapRegistryError> {
        let mut longest_match: Option<(usize, Vec<String>)> = None;
        let Self::RdapBootstrapRegistry(bootstrap) = self;
        for service in &bootstrap.services {
            let tlds = service
                .first()
                .ok_or(BootstrapRegistryError::EmptyService)?;
            for tld in tlds {
                // if the ldh domain ends with the tld or the tld is the empty string which means the root
                if ldh.ends_with(tld) || tld.is_empty() {
                    let urls = service.last().ok_or(BootstrapRegistryError::EmptyUrlSet)?;
                    let longest = longest_match.get_or_insert_with(|| (tld.len(), urls.to_owned()));
                    if longest.0 < tld.len() {
                        *longest = (tld.len(), urls.to_owned());
                    }
                }
            }
        }
        let longest = longest_match.ok_or(BootstrapRegistryError::NoBootstrapUrls)?;
        Ok(longest.1)
    }

    /// Get the URLS from the IANA autnum bootstrap registry.
    fn get_asn_bootstrap_urls(&self, asn: &str) -> Result<Vec<String>, BootstrapRegistryError> {
        let autnum = asn
            .trim_start_matches(|c| -> bool { matches!(c, 'a' | 'A' | 's' | 'S') })
            .parse::<u32>()
            .map_err(|_| BootstrapRegistryError::InvalidBootstrapInput)?;
        let Self::RdapBootstrapRegistry(bootstrap) = self;
        for service in &bootstrap.services {
            let as_ranges = service
                .first()
                .ok_or(BootstrapRegistryError::EmptyService)?;
            for range in as_ranges {
                let as_split = range.split('-').collect::<Vec<&str>>();
                let start_as = as_split
                    .first()
                    .ok_or(BootstrapRegistryError::InvalidBootstrapService)?
                    .parse::<u32>()
                    .map_err(|_| BootstrapRegistryError::InvalidBootstrapInput)?;
                let end_as = as_split
                    .last()
                    .ok_or(BootstrapRegistryError::InvalidBootstrapService)?
                    .parse::<u32>()
                    .map_err(|_| BootstrapRegistryError::InvalidBootstrapService)?;
                if start_as <= autnum && end_as >= autnum {
                    let urls = service.last().ok_or(BootstrapRegistryError::EmptyUrlSet)?;
                    return Ok(urls.to_owned());
                }
            }
        }
        Err(BootstrapRegistryError::NoBootstrapUrls)
    }

    /// Get the URLs from the IANA IPv4 bootstrap registry.
    fn get_ipv4_bootstrap_urls(&self, ipv4: &str) -> Result<Vec<String>, BootstrapRegistryError> {
        let mut pm: PrefixMap<Ipv4Net, Vec<String>> = PrefixMap::new();
        let Self::RdapBootstrapRegistry(bootstrap) = self;
        for service in &bootstrap.services {
            let urls = service.last().ok_or(BootstrapRegistryError::EmptyService)?;
            for cidr in service
                .first()
                .ok_or(BootstrapRegistryError::InvalidBootstrapService)?
            {
                pm.insert(
                    cidr.parse()
                        .map_err(|_| BootstrapRegistryError::InvalidBootstrapService)?,
                    urls.clone(),
                );
            }
        }
        let net = pm
            .get_lpm(
                &ipv4
                    .parse::<Ipv4Net>()
                    .map_err(|_| BootstrapRegistryError::InvalidBootstrapInput)?,
            )
            .ok_or(BootstrapRegistryError::NoBootstrapUrls)?;
        Ok(net.1.to_owned())
    }

    /// Get the URLs from the IANA IPv6 bootstrap registry.
    fn get_ipv6_bootstrap_urls(&self, ipv6: &str) -> Result<Vec<String>, BootstrapRegistryError> {
        let mut pm: PrefixMap<Ipv6Net, Vec<String>> = PrefixMap::new();
        let Self::RdapBootstrapRegistry(bootstrap) = self;
        for service in &bootstrap.services {
            let urls = service.last().ok_or(BootstrapRegistryError::EmptyService)?;
            for cidr in service
                .first()
                .ok_or(BootstrapRegistryError::InvalidBootstrapService)?
            {
                pm.insert(
                    cidr.parse()
                        .map_err(|_| BootstrapRegistryError::InvalidBootstrapService)?,
                    urls.clone(),
                );
            }
        }
        let net = pm
            .get_lpm(
                &ipv6
                    .parse::<Ipv6Net>()
                    .map_err(|_| BootstrapRegistryError::InvalidBootstrapInput)?,
            )
            .ok_or(BootstrapRegistryError::NoBootstrapUrls)?;
        Ok(net.1.to_owned())
    }

    /// Get the URLs from the IANA object tag bootstrap registry.
    fn get_tag_bootstrap_urls(&self, tag: &str) -> Result<Vec<String>, BootstrapRegistryError> {
        let Self::RdapBootstrapRegistry(bootstrap) = self;
        for service in &bootstrap.services {
            let object_tag = service
                .get(1)
                .ok_or(BootstrapRegistryError::InvalidBootstrapService)?
                .first()
                .ok_or(BootstrapRegistryError::EmptyService)?;
            if object_tag.eq_ignore_ascii_case(tag) {
                let urls = service.last().ok_or(BootstrapRegistryError::EmptyUrlSet)?;
                return Ok(urls.to_owned());
            }
        }
        Err(BootstrapRegistryError::NoBootstrapUrls)
    }
}

/// Prefer HTTPS urls.
pub fn get_preferred_url(urls: Vec<String>) -> Result<String, BootstrapRegistryError> {
    if urls.is_empty() {
        Err(BootstrapRegistryError::EmptyUrlSet)
    } else {
        let url = urls
            .iter()
            .find(|s| s.starts_with("https://"))
            .unwrap_or_else(|| urls.first().unwrap());
        Ok(url.to_owned())
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use rstest::rstest;

    use crate::iana::{get_preferred_url, BootstrapRegistry};

    use super::{IanaRegistry, IanaRegistryType};

    #[rstest]
    #[case(IanaRegistryType::RdapBootstrapDns, "dns.json")]
    #[case(IanaRegistryType::RdapBootstrapAsn, "asn.json")]
    #[case(IanaRegistryType::RdapBootstrapIpv4, "ipv4.json")]
    #[case(IanaRegistryType::RdapBootstrapIpv6, "ipv6.json")]
    #[case(IanaRegistryType::RdapObjectTags, "object-tags.json")]
    fn GIVEN_registry_WHEN_get_file_name_THEN_correct_result(
        #[case] registry: IanaRegistryType,
        #[case] expected: &str,
    ) {
        // GIVEN in parameters

        // WHEN
        let actual = registry.file_name();

        // THEN
        assert_eq!(actual, expected);
    }

    #[test]
    fn GIVEN_domain_bootstrap_WHEN_deserialize_THEN_success() {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "Some text",
                "services": [
                  [
                    ["net", "com"],
                    [
                      "https://registry.example.com/myrdap/"
                    ]
                  ],
                  [
                    ["org", "mytld"],
                    [
                      "https://example.org/"
                    ]
                  ],
                  [
                    ["xn--zckzah"],
                    [
                      "https://example.net/rdap/xn--zckzah/",
                      "http://example.net/rdap/xn--zckzah/"
                    ]
                  ]
                ]
            }
        "#;

        // WHEN
        let actual = serde_json::from_str::<IanaRegistry>(bootstrap);

        // THEN
        actual.unwrap();
    }

    #[test]
    fn GIVEN_one_url_WHEN_preferred_urls_THEN_that_is_the_one() {
        // GIVEN
        let urls = vec!["http://foo.example".to_string()];

        // WHEN
        let actual = get_preferred_url(urls).expect("cannot get preferred url");

        // THEN
        assert_eq!(actual, "http://foo.example");
    }

    #[test]
    fn GIVEN_one_http_and_https_url_WHEN_preferred_urls_THEN_return_https() {
        // GIVEN
        let urls = vec![
            "http://foo.example".to_string(),
            "https://foo.example".to_string(),
        ];

        // WHEN
        let actual = get_preferred_url(urls).expect("cannot get preferred url");

        // THEN
        assert_eq!(actual, "https://foo.example");
    }

    #[test]
    fn GIVEN_domain_bootstrap_with_matching_WHEN_find_THEN_url_matches() {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "Some text",
                "services": [
                  [
                    ["net", "com"],
                    [
                      "https://registry.example.com/myrdap/"
                    ]
                  ],
                  [
                    ["org", "mytld"],
                    [
                      "https://example.org/"
                    ]
                  ]
                ]
            }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse domain bootstrap");

        // WHEN
        let actual = iana.get_dns_bootstrap_urls("foo.org");

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            "https://example.org/"
        );
    }

    #[test]
    fn GIVEN_domain_bootstrap_with_two_matching_WHEN_find_THEN_return_longest_match() {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "Some text",
                "services": [
                  [
                    ["co.uk"],
                    [
                      "https://registry.co.uk/"
                    ]
                  ],
                  [
                    ["uk"],
                    [
                      "https://registry.uk/"
                    ]
                  ]
                ]
            }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse domain bootstrap");

        // WHEN
        let actual = iana.get_dns_bootstrap_urls("foo.co.uk");

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            "https://registry.co.uk/"
        );
    }

    #[test]
    fn GIVEN_domain_bootstrap_with_root_WHEN_find_THEN_url_matches() {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "Some text",
                "services": [
                  [
                    ["net", "com"],
                    [
                      "https://registry.example.com/myrdap/"
                    ]
                  ],
                  [
                    [""],
                    [
                      "https://example.org/"
                    ]
                  ]
                ]
            }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse domain bootstrap");

        // WHEN
        let actual = iana.get_dns_bootstrap_urls("foo.org");

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            "https://example.org/"
        );
    }

    #[test]
    fn GIVEN_autnum_bootstrap_with_match_WHEN_find_with_string_THEN_return_match() {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "RDAP Bootstrap file for example registries.",
                "services": [
                  [
                    ["64496-64496"],
                    [
                      "https://rir3.example.com/myrdap/"
                    ]
                  ],
                  [
                    ["64497-64510", "65536-65551"],
                    [
                      "https://example.org/"
                    ]
                  ],
                  [
                    ["64512-65534"],
                    [
                      "http://example.net/rdaprir2/",
                      "https://example.net/rdaprir2/"
                    ]
                  ]
                ]
            }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse autnum bootstrap");

        // WHEN
        let actual = iana.get_asn_bootstrap_urls("as64498");

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            "https://example.org/"
        );
    }

    #[rstest]
    #[case(64497u32, "https://example.org/")]
    #[case(64498u32, "https://example.org/")]
    #[case(64510u32, "https://example.org/")]
    #[case(65536u32, "https://example.org/")]
    #[case(65537u32, "https://example.org/")]
    #[case(64513u32, "http://example.net/rdaprir2/")]
    fn GIVEN_autnum_bootstrap_with_match_WHEN_find_with_number_THEN_return_match(
        #[case] asn: u32,
        #[case] bootstrap_url: &str,
    ) {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "RDAP Bootstrap file for example registries.",
                "services": [
                  [
                    ["64496-64496"],
                    [
                      "https://rir3.example.com/myrdap/"
                    ]
                  ],
                  [
                    ["64497-64510", "65536-65551"],
                    [
                      "https://example.org/"
                    ]
                  ],
                  [
                    ["64512-65534"],
                    [
                      "http://example.net/rdaprir2/",
                      "https://example.net/rdaprir2/"
                    ]
                  ]
                ]
            }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse autnum bootstrap");

        // WHEN
        let actual = iana.get_asn_bootstrap_urls(&asn.to_string());

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            bootstrap_url
        );
    }

    #[test]
    fn GIVEN_ipv4_bootstrap_with_match_WHEN_find_with_ip_address_THEN_return_match() {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "RDAP Bootstrap file for example registries.",
                "services": [
                  [
                    ["198.51.100.0/24", "192.0.0.0/8"],
                    [
                      "https://rir1.example.com/myrdap/"
                    ]
                  ],
                  [
                    ["203.0.113.0/24", "192.0.2.0/24"],
                    [
                      "https://example.org/"
                    ]
                  ],
                  [
                    ["203.0.113.0/28"],
                    [
                      "https://example.net/rdaprir2/",
                      "http://example.net/rdaprir2/"
                    ]
                  ]
                ]
            }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse ipv4 bootstrap");

        // WHEN
        let actual = iana.get_ipv4_bootstrap_urls("198.51.100.1/32");

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            "https://rir1.example.com/myrdap/"
        );
    }

    #[test]
    fn GIVEN_ipv4_bootstrap_with_match_WHEN_find_with_cidr_THEN_return_match() {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "RDAP Bootstrap file for example registries.",
                "services": [
                  [
                    ["198.51.100.0/24", "192.0.0.0/8"],
                    [
                      "https://rir1.example.com/myrdap/"
                    ]
                  ],
                  [
                    ["203.0.113.0/24", "192.0.2.0/24"],
                    [
                      "https://example.org/"
                    ]
                  ],
                  [
                    ["203.0.113.0/28"],
                    [
                      "https://example.net/rdaprir2/",
                      "http://example.net/rdaprir2/"
                    ]
                  ]
                ]
            }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse ipv4 bootstrap");

        // WHEN
        let actual = iana.get_ipv4_bootstrap_urls("203.0.113.0/24");

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            "https://example.org/"
        );
    }

    #[test]
    fn GIVEN_ipv6_bootstrap_with_match_WHEN_find_with_ip_address_THEN_return_match() {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "RDAP Bootstrap file for example registries.",
                "services": [
                  [
                    ["2001:db8::/34"],
                    [
                      "https://rir2.example.com/myrdap/"
                    ]
                  ],
                  [
                    ["2001:db8:4000::/36", "2001:db8:ffff::/48"],
                    [
                      "https://example.org/"
                    ]
                  ],
                  [
                    ["2001:db8:1000::/36"],
                    [
                      "https://example.net/rdaprir2/",
                      "http://example.net/rdaprir2/"
                    ]
                  ]
                ]
            }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse ipv6 bootstrap");

        // WHEN
        let actual = iana.get_ipv6_bootstrap_urls("2001:db8::1/128");

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            "https://rir2.example.com/myrdap/"
        );
    }

    #[test]
    fn GIVEN_ipv6_bootstrap_with_match_WHEN_find_with_ip_cidr_THEN_return_match() {
        // GIVEN
        let bootstrap = r#"
            {
                "version": "1.0",
                "publication": "2024-01-07T10:11:12Z",
                "description": "RDAP Bootstrap file for example registries.",
                "services": [
                  [
                    ["2001:db8::/34"],
                    [
                      "https://rir2.example.com/myrdap/"
                    ]
                  ],
                  [
                    ["2001:db8:4000::/36", "2001:db8:ffff::/48"],
                    [
                      "https://example.org/"
                    ]
                  ],
                  [
                    ["2001:db8:1000::/36"],
                    [
                      "https://example.net/rdaprir2/",
                      "http://example.net/rdaprir2/"
                    ]
                  ]
                ]
            }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse ipv6 bootstrap");

        // WHEN
        let actual = iana.get_ipv6_bootstrap_urls("2001:db8:4000::/36");

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            "https://example.org/"
        );
    }

    #[test]
    fn GIVEN_tag_bootstrap_with_match_WHEN_find_with_tag_THEN_return_match() {
        // GIVEN
        let bootstrap = r#"
            {
              "version": "1.0",
              "publication": "YYYY-MM-DDTHH:MM:SSZ",
              "description": "RDAP bootstrap file for service provider object tags",
              "services": [
                [
                  ["contact@example.com"],
                  ["YYYY"],
                  [
                    "https://example.com/rdap/"
                  ]
                ],
                [
                  ["contact@example.org"],
                  ["ZZ54"],
                  [
                    "http://rdap.example.org/"
                  ]
                ],
                [
                  ["contact@example.net"],
                  ["1754"],
                  [
                    "https://example.net/rdap/",
                    "http://example.net/rdap/"
                  ]
                ]
              ]
             }
        "#;
        let iana =
            serde_json::from_str::<IanaRegistry>(bootstrap).expect("cannot parse tag bootstrap");

        // WHEN
        let actual = iana.get_tag_bootstrap_urls("YYYY");

        // THEN
        assert_eq!(
            actual.expect("no vec").first().expect("vec is empty"),
            "https://example.com/rdap/"
        );
    }
}
