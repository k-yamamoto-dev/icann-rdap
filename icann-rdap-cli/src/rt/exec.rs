//! Function to execute tests.

use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use {
    hickory_client::{
        client::{AsyncClient, ClientConnection, ClientHandle},
        rr::{DNSClass, Name, RecordType},
        udp::UdpClientConnection,
    },
    icann_rdap_client::{
        http::{create_client, create_client_with_addr, ClientConfig},
        iana::{qtype_to_bootstrap_url, BootstrapStore},
        rdap::{rdap_url_request, QueryType},
        RdapClientError,
    },
    icann_rdap_common::response::{get_related_links, ExtensionId},
    reqwest::{header::HeaderValue, Url},
    thiserror::Error,
    tracing::{debug, info},
    url::ParseError,
};

use crate::rt::results::{RunFeature, TestRun};

use super::results::{DnsData, TestResults};

#[derive(Default)]
pub struct TestOptions {
    pub skip_v4: bool,
    pub skip_v6: bool,
    pub skip_origin: bool,
    pub origin_value: String,
    pub chase_referral: bool,
    pub expect_extensions: Vec<String>,
    pub expect_groups: Vec<ExtensionGroup>,
    pub allow_unregistered_extensions: bool,
    pub one_addr: bool,
    pub dns_resolver: Option<String>,
}

#[derive(Clone)]
pub enum ExtensionGroup {
    Gtld,
    Nro,
    NroAsn,
}

#[derive(Debug, Error)]
pub enum TestExecutionError {
    #[error(transparent)]
    RdapClient(#[from] RdapClientError),
    #[error(transparent)]
    UrlParseError(#[from] ParseError),
    #[error(transparent)]
    AddrParseError(#[from] std::net::AddrParseError),
    #[error("No host to resolve")]
    NoHostToResolve,
    #[error("No rdata")]
    NoRdata,
    #[error("Bad rdata")]
    BadRdata,
    #[error(transparent)]
    Client(#[from] reqwest::Error),
    #[error(transparent)]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
    #[error("Unsupporte Query Type")]
    UnsupportedQueryType,
    #[error("No referral to chase")]
    NoReferralToChase,
    #[error("Unregistered extension")]
    UnregisteredExtension,
}

pub async fn execute_tests<BS: BootstrapStore>(
    bs: &BS,
    value: &QueryType,
    options: &TestOptions,
    client_config: &ClientConfig,
) -> Result<TestResults, TestExecutionError> {
    let bs_client = create_client(client_config)?;

    // normalize extensions
    let extensions = normalize_extension_ids(options)?;
    let options = &TestOptions {
        expect_extensions: extensions,
        expect_groups: options.expect_groups.clone(),
        origin_value: options.origin_value.clone(),
        dns_resolver: options.dns_resolver.clone(),
        ..*options
    };

    // get the query url
    let mut query_url = match value {
        QueryType::Help => return Err(TestExecutionError::UnsupportedQueryType),
        QueryType::Url(url) => url.to_owned(),
        _ => {
            let base_url = qtype_to_bootstrap_url(&bs_client, bs, value, |reg| {
                info!("Fetching IANA registry {} for value {value}", reg.url())
            })
            .await?;
            value.query_url(&base_url)?
        }
    };
    // if the URL to test is a referral
    if options.chase_referral {
        let client = create_client(client_config)?;
        info!("Fetching referral from {query_url}");
        let response_data = rdap_url_request(&query_url, &client).await?;
        query_url = get_related_links(&response_data.rdap)
            .first()
            .ok_or(TestExecutionError::NoReferralToChase)?
            .to_string();
        info!("Referral is {query_url}");
    }

    let parsed_url = Url::parse(&query_url)?;
    let port = parsed_url.port().unwrap_or_else(|| {
        if parsed_url.scheme().eq("https") {
            443
        } else {
            80
        }
    });
    let host = parsed_url
        .host_str()
        .ok_or(TestExecutionError::NoHostToResolve)?;

    info!("Testing {query_url}");
    let dns_data = get_dns_records(host, options).await?;
    let mut test_results = TestResults::new(query_url.clone(), dns_data.clone());

    let mut more_runs = true;
    for v4 in dns_data.v4_addrs {
        // test run without origin
        let mut test_run = TestRun::new_v4(vec![], v4, port);
        if !options.skip_v4 && more_runs {
            let client = create_client_with_addr(client_config, host, test_run.socket_addr)?;
            info!("Sending request to {}", test_run.socket_addr);
            let rdap_response = rdap_url_request(&query_url, &client).await;
            test_run = test_run.end(rdap_response, options);
        }
        test_results.add_test_run(test_run);

        // test run with origin
        let mut test_run = TestRun::new_v4(vec![RunFeature::OriginHeader], v4, port);
        if !options.skip_v4 && !options.skip_origin && more_runs {
            let client_config = ClientConfig::from_config(client_config)
                .origin(HeaderValue::from_str(&options.origin_value)?)
                .build();
            let client = create_client_with_addr(&client_config, host, test_run.socket_addr)?;
            info!("Sending request to {}", test_run.socket_addr);
            let rdap_response = rdap_url_request(&query_url, &client).await;
            test_run = test_run.end(rdap_response, options);
        }
        test_results.add_test_run(test_run);
        if options.one_addr {
            more_runs = false;
        }
    }

    let mut more_runs = true;
    for v6 in dns_data.v6_addrs {
        // test run without origin
        let mut test_run = TestRun::new_v6(vec![], v6, port);
        if !options.skip_v6 && more_runs {
            let client = create_client_with_addr(client_config, host, test_run.socket_addr)?;
            info!("Sending request to {}", test_run.socket_addr);
            let rdap_response = rdap_url_request(&query_url, &client).await;
            test_run = test_run.end(rdap_response, options);
        }
        test_results.add_test_run(test_run);

        // test run with origin
        let mut test_run = TestRun::new_v6(vec![RunFeature::OriginHeader], v6, port);
        if !options.skip_v6 && !options.skip_origin && more_runs {
            let client_config = ClientConfig::from_config(client_config)
                .origin(HeaderValue::from_str(&options.origin_value)?)
                .build();
            let client = create_client_with_addr(&client_config, host, test_run.socket_addr)?;
            info!("Sending request to {}", test_run.socket_addr);
            let rdap_response = rdap_url_request(&query_url, &client).await;
            test_run = test_run.end(rdap_response, options);
        }
        test_results.add_test_run(test_run);
        if options.one_addr {
            more_runs = false;
        }
    }

    test_results.end(options);
    info!("Testing complete.");
    Ok(test_results)
}

async fn get_dns_records(host: &str, options: &TestOptions) -> Result<DnsData, TestExecutionError> {
    // short circuit dns if these are ip addresses
    if let Ok(ip4) = Ipv4Addr::from_str(host) {
        return Ok(DnsData {
            v4_cname: None,
            v6_cname: None,
            v4_addrs: vec![ip4],
            v6_addrs: vec![],
        });
    } else if let Ok(ip6) = Ipv6Addr::from_str(host.trim_start_matches('[').trim_end_matches(']')) {
        return Ok(DnsData {
            v4_cname: None,
            v6_cname: None,
            v4_addrs: vec![],
            v6_addrs: vec![ip6],
        });
    }

    let def_dns_resolver = "8.8.8.8:53".to_string();
    let dns_resolver = options.dns_resolver.as_ref().unwrap_or(&def_dns_resolver);
    let conn = UdpClientConnection::new(dns_resolver.parse()?)
        .unwrap()
        .new_stream(None);
    let (mut client, bg) = AsyncClient::connect(conn).await.unwrap();

    // make sure to run the background task
    tokio::spawn(bg);

    let mut dns_data = DnsData::default();

    // Create a query future
    let query = client.query(Name::from_str(host).unwrap(), DNSClass::IN, RecordType::A);

    // wait for its response
    let response = query.await.unwrap();

    for answer in response.answers() {
        match answer.record_type() {
            RecordType::CNAME => {
                let cname = answer
                    .data()
                    .ok_or(TestExecutionError::NoRdata)?
                    .clone()
                    .into_cname()
                    .map_err(|_e| TestExecutionError::BadRdata)?
                    .0
                    .to_string();
                debug!("Found cname {cname}");
                dns_data.v4_cname = Some(cname);
            }
            RecordType::A => {
                let addr = answer
                    .data()
                    .ok_or(TestExecutionError::NoRdata)?
                    .clone()
                    .into_a()
                    .map_err(|_e| TestExecutionError::BadRdata)?
                    .0;
                debug!("Found IPv4 {addr}");
                dns_data.v4_addrs.push(addr);
            }
            _ => {
                // do nothing
            }
        };
    }

    // Create a query future
    let query = client.query(
        Name::from_str(host).unwrap(),
        DNSClass::IN,
        RecordType::AAAA,
    );

    // wait for its response
    let response = query.await.unwrap();

    for answer in response.answers() {
        match answer.record_type() {
            RecordType::CNAME => {
                let cname = answer
                    .data()
                    .ok_or(TestExecutionError::NoRdata)?
                    .clone()
                    .into_cname()
                    .map_err(|_e| TestExecutionError::BadRdata)?
                    .0
                    .to_string();
                debug!("Found cname {cname}");
                dns_data.v6_cname = Some(cname);
            }
            RecordType::AAAA => {
                let addr = answer
                    .data()
                    .ok_or(TestExecutionError::NoRdata)?
                    .clone()
                    .into_aaaa()
                    .map_err(|_e| TestExecutionError::BadRdata)?
                    .0;
                debug!("Found IPv6 {addr}");
                dns_data.v6_addrs.push(addr);
            }
            _ => {
                // do nothing
            }
        };
    }

    Ok(dns_data)
}

fn normalize_extension_ids(options: &TestOptions) -> Result<Vec<String>, TestExecutionError> {
    let mut retval = options.expect_extensions.clone();

    // check for unregistered extensions
    if !options.allow_unregistered_extensions {
        for ext in &retval {
            if ExtensionId::from_str(ext).is_err() {
                return Err(TestExecutionError::UnregisteredExtension);
            }
        }
    }

    // put the groups in
    for group in &options.expect_groups {
        match group {
            ExtensionGroup::Gtld => {
                retval.push(format!(
                    "{}|{}",
                    ExtensionId::IcannRdapResponseProfile0,
                    ExtensionId::IcannRdapResponseProfile1
                ));
                retval.push(format!(
                    "{}|{}",
                    ExtensionId::IcannRdapTechnicalImplementationGuide0,
                    ExtensionId::IcannRdapTechnicalImplementationGuide1
                ));
            }
            ExtensionGroup::Nro => {
                retval.push(ExtensionId::NroRdapProfile0.to_string());
                retval.push(ExtensionId::Cidr0.to_string());
            }
            ExtensionGroup::NroAsn => {
                retval.push(ExtensionId::NroRdapProfile0.to_string());
                retval.push(format!(
                    "{}|{}",
                    ExtensionId::NroRdapProfileAsnFlat0,
                    ExtensionId::NroRdapProfileAsnHierarchical0
                ));
            }
        }
    }
    Ok(retval)
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use icann_rdap_common::response::ExtensionId;

    use crate::rt::exec::{ExtensionGroup, TestOptions};

    use super::normalize_extension_ids;

    #[test]
    fn GIVEN_gtld_WHEN_normalize_extensions_THEN_list_contains_gtld_ids() {
        // GIVEN
        let given = vec![ExtensionGroup::Gtld];

        // WHEN
        let options = TestOptions {
            expect_groups: given,
            ..Default::default()
        };
        let actual = normalize_extension_ids(&options).unwrap();

        // THEN
        let expected1 = format!(
            "{}|{}",
            ExtensionId::IcannRdapResponseProfile0,
            ExtensionId::IcannRdapResponseProfile1
        );
        assert!(actual.contains(&expected1));

        let expected2 = format!(
            "{}|{}",
            ExtensionId::IcannRdapTechnicalImplementationGuide0,
            ExtensionId::IcannRdapTechnicalImplementationGuide1
        );
        assert!(actual.contains(&expected2));
    }

    #[test]
    fn GIVEN_nro_and_foo_WHEN_normalize_extensions_THEN_list_contains_nro_ids_and_foo() {
        // GIVEN
        let groups = vec![ExtensionGroup::Nro];
        let exts = vec!["foo1".to_string()];

        // WHEN
        let options = TestOptions {
            allow_unregistered_extensions: true,
            expect_extensions: exts,
            expect_groups: groups,
            ..Default::default()
        };
        let actual = normalize_extension_ids(&options).unwrap();
        dbg!(&actual);

        // THEN
        assert!(actual.contains(&ExtensionId::NroRdapProfile0.to_string()));
        assert!(actual.contains(&ExtensionId::Cidr0.to_string()));
        assert!(actual.contains(&"foo1".to_string()));
    }

    #[test]
    fn GIVEN_nro_and_foo_WHEN_unreg_disallowed_THEN_err() {
        // GIVEN
        let groups = vec![ExtensionGroup::Nro];
        let exts = vec!["foo1".to_string()];

        // WHEN
        let options = TestOptions {
            expect_extensions: exts,
            expect_groups: groups,
            ..Default::default()
        };
        let actual = normalize_extension_ids(&options);

        // THEN
        assert!(actual.is_err())
    }

    #[test]
    fn GIVEN_unregistered_ext_WHEN_normalize_extensions_THEN_error() {
        // GIVEN
        let given = vec!["foo".to_string()];

        // WHEN
        let options = TestOptions {
            expect_extensions: given,
            ..Default::default()
        };
        let actual = normalize_extension_ids(&options);

        // THEN
        assert!(actual.is_err());
    }

    #[test]
    fn GIVEN_unregistered_ext_WHEN_allowed_THEN_no_error() {
        // GIVEN
        let given = vec!["foo".to_string()];

        // WHEN
        let options = TestOptions {
            expect_extensions: given,
            allow_unregistered_extensions: true,
            ..Default::default()
        };
        let actual = normalize_extension_ids(&options);

        // THEN
        assert!(actual.is_ok());
    }
}
