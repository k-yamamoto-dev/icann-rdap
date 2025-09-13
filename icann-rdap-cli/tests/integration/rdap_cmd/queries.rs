use {
    icann_rdap_common::{
        prelude::Link,
        response::{Autnum, Domain, Entity, Nameserver, Network},
    },
    icann_rdap_srv::storage::StoreOps,
    rstest::rstest,
};

use crate::test_jig::TestJig;

#[rstest]
#[case("foo.example", "foo.example")]
#[case("foo.example", "foo.example.")]
#[case("foo.example", "FOO.EXAMPLE")]
#[case("foó.example", "foó.example")] // unicode
#[tokio::test(flavor = "multi_thread")]
async fn test_domain_queries(#[case] db_domain: &str, #[case] q_domain: &str) {
    // GIVEN domain
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(&Domain::builder().ldh_name(db_domain).build())
        .await
        .expect("add domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query
    test_jig.cmd.arg(q_domain);

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_tld_query() {
    // GIVEN tld to query
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(&Domain::builder().ldh_name("example").build())
        .await
        .expect("add domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN queried
    // without "--tld-lookup=none" then this attempts to query IANA instead of the test server
    test_jig.cmd.arg("--tld-lookup=none").arg(".example");

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_entity_query() {
    // GIVEN entity
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_entity(&Entity::builder().handle("foo").build())
        .await
        .expect("add entity in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query
    test_jig.cmd.arg("foo");

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_nameserver_query() {
    // GIVEN nameserver
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_nameserver(
        &Nameserver::builder()
            .ldh_name("ns.foo.example")
            .build()
            .unwrap(),
    )
    .await
    .expect("add nameserver in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query
    test_jig.cmd.arg("ns.foo.example");

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_autnum_query() {
    // GIVEN autnum
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_autnum(&Autnum::builder().autnum_range(700..710).build())
        .await
        .expect("add autnum in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query
    test_jig.cmd.arg("700");

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_network_ip_query() {
    // GIVEN network
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_network(
        &Network::builder()
            .cidr("10.0.0.0/24")
            .build()
            .expect("cidr parsing"),
    )
    .await
    .expect("add network in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query ip address
    test_jig.cmd.arg("10.0.0.1");

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[rstest]
#[case("10.0.0.0/24", "10.0.0.0/24")]
#[case("10.0.0.0/24", "10.0.0/24")]
#[tokio::test(flavor = "multi_thread")]
async fn test_network_cidr_query(#[case] db_cidr: &str, #[case] q_cidr: &str) {
    // GIVEN network
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_network(
        &Network::builder()
            .cidr(db_cidr)
            .build()
            .expect("cidr parsing"),
    )
    .await
    .expect("add network in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query by CIDR
    test_jig.cmd.arg(q_cidr);

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_url_query() {
    // GIVEN url
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(&Domain::builder().ldh_name("foo.example").build())
        .await
        .expect("add domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN queried
    let url = format!("{}/domain/foo.example", test_jig.rdap_base);
    test_jig.cmd.arg(url);

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_idn_query_a_label() {
    // GIVEN idn
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(&Domain::builder().ldh_name("xn--caf-dma.example").build())
        .await
        .expect("add domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query alabel
    test_jig.cmd.arg("-t").arg("a-label").arg("café.example");

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_search() {
    // GIVEN domain
    let mut test_jig = TestJig::new_rdap_with_dn_search().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(&Domain::builder().ldh_name("foo.example").build())
        .await
        .expect("add domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN search for the domain
    test_jig.cmd.arg("-t").arg("domain-name").arg("foo.*");

    // THEN success
    let assert = test_jig.cmd.assert();
    assert.success();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_with_status_output_text() {
    // GIVEN domain with status
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("foo.example")
            .status("client delete prohibited")
            .status("client transfer prohibited")
            .status("client update prohibited")
            .build(),
    )
    .await
    .expect("add domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with status-text output type
    test_jig.cmd.arg("foo.example").arg("-O").arg("status-text");

    // THEN output is text of status
    let assert = test_jig.cmd.assert();
    assert
        .success()
        .stdout("client delete prohibited\nclient transfer prohibited\nclient update prohibited\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_referral_with_status_output_text() {
    // GIVEN domain that refers to another domain (e.g. registry -> registrar)
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("foo.example")
            .link(
                Link::builder()
                    .rel("related")
                    // note that in real life this would be a foo.example referrign to a foo.example
                    // in another server. However, to get this to work with one server, we
                    // refer foo.example to bar.example.
                    .href(format!("{}/domain/bar.example", test_jig.rdap_base))
                    .value(format!("{}/domain/foo.example", test_jig.rdap_base))
                    .build(),
            )
            .status("client delete prohibited")
            .status("client transfer prohibited")
            .status("client update prohibited")
            .build(),
    )
    .await
    .expect("add foo domain in tx");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("bar.example")
            .status("server delete prohibited")
            .status("server transfer prohibited")
            .status("server update prohibited")
            .build(),
    )
    .await
    .expect("add bar domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with status-text output type
    test_jig.cmd.arg("foo.example").arg("-O").arg("status-text");

    // THEN output is text of status
    let assert = test_jig.cmd.assert();
    assert.success().stdout(
        r#"client delete prohibited
client transfer prohibited
client update prohibited
server delete prohibited
server transfer prohibited
server update prohibited
"#,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_with_status_output_json() {
    // GIVEN domain with status
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("bar.example")
            .status("client delete prohibited")
            .status("client transfer prohibited")
            .status("client update prohibited")
            .build(),
    )
    .await
    .expect("add domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with status-json output type
    test_jig.cmd.arg("bar.example").arg("-O").arg("status-json");

    // THEN output type is json with status
    let assert = test_jig.cmd.assert();
    assert
        .success()
        .stdout(
            "{\"status\":[\"client delete prohibited\",\"client transfer prohibited\",\"client update prohibited\"]}\n",
        );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_referral_with_status_output_json() {
    // GIVEN domain that refers to another domain (e.g. registry -> registrar)
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("foo.example")
            .link(
                Link::builder()
                    .rel("related")
                    // note that in real life this would be a foo.example referrign to a foo.example
                    // in another server. However, to get this to work with one server, we
                    // refer foo.example to bar.example.
                    .href(format!("{}/domain/bar.example", test_jig.rdap_base))
                    .value(format!("{}/domain/foo.example", test_jig.rdap_base))
                    .build(),
            )
            .status("client delete prohibited")
            .build(),
    )
    .await
    .expect("add foo domain in tx");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("bar.example")
            .status("server delete prohibited")
            .build(),
    )
    .await
    .expect("add bar domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with status-json output type
    test_jig.cmd.arg("foo.example").arg("-O").arg("status-json");

    // THEN output type is json with status
    let assert = test_jig.cmd.assert();
    assert
        .success()
        .stdout("{\"status\":[\"client delete prohibited\",\"server delete prohibited\"]}\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_with_referral_for_only_registry_with_status_output_json() {
    // GIVEN domain that refers to another domain (e.g. registry -> registrar)
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("foo.example")
            .link(
                Link::builder()
                    .rel("related")
                    // note that in real life this would be a foo.example referrign to a foo.example
                    // in another server. However, to get this to work with one server, we
                    // refer foo.example to bar.example.
                    .href(format!("{}/domain/bar.example", test_jig.rdap_base))
                    .value(format!("{}/domain/foo.example", test_jig.rdap_base))
                    .build(),
            )
            .status("client delete prohibited")
            .build(),
    )
    .await
    .expect("add foo domain in tx");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("bar.example")
            .status("server delete prohibited")
            .build(),
    )
    .await
    .expect("add bar domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with status-json output type and ask for registry only
    test_jig
        .cmd
        .arg("foo.example")
        .arg("-O")
        .arg("status-json")
        .arg("-p")
        .arg("registry");

    // THEN output type is json with status
    let assert = test_jig.cmd.assert();
    assert
        .success()
        .stdout("{\"status\":[\"client delete prohibited\"]}\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_with_referral_for_only_registrar_with_status_output_json() {
    // GIVEN domain that refers to another domain (e.g. registry -> registrar)
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("foo.example")
            .link(
                Link::builder()
                    .rel("related")
                    // note that in real life this would be a foo.example referrign to a foo.example
                    // in another server. However, to get this to work with one server, we
                    // refer foo.example to bar.example.
                    .href(format!("{}/domain/bar.example", test_jig.rdap_base))
                    .value(format!("{}/domain/foo.example", test_jig.rdap_base))
                    .build(),
            )
            .status("client delete prohibited")
            .build(),
    )
    .await
    .expect("add foo domain in tx");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("bar.example")
            .status("server delete prohibited")
            .build(),
    )
    .await
    .expect("add bar domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with status-json output type and ask for registry only
    test_jig
        .cmd
        .arg("foo.example")
        .arg("-O")
        .arg("status-json")
        .arg("-p")
        .arg("registrar");

    // THEN output type is json with status
    let assert = test_jig.cmd.assert();
    assert
        .success()
        .stdout("{\"status\":[\"server delete prohibited\"]}\n");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_with_url_output() {
    // GIVEN domain
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(&Domain::builder().ldh_name("bar.example").build())
        .await
        .expect("add domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with url output type
    test_jig.cmd.arg("bar.example").arg("-O").arg("url");

    // THEN output type is the urls
    let assert = test_jig.cmd.assert();
    assert
        .success()
        .stdout(format!("{}/domain/bar.example\n", test_jig.rdap_base));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_referral_with_url_output() {
    // GIVEN domain that refers to another domain (e.g. registry -> registrar)
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("foo.example")
            .link(
                Link::builder()
                    .rel("related")
                    // note that in real life this would be a foo.example referrign to a foo.example
                    // in another server. However, to get this to work with one server, we
                    // refer foo.example to bar.example.
                    .href(format!("{}/domain/bar.example", test_jig.rdap_base))
                    .value(format!("{}/domain/foo.example", test_jig.rdap_base))
                    .build(),
            )
            .build(),
    )
    .await
    .expect("add foo domain in tx");
    tx.add_domain(&Domain::builder().ldh_name("bar.example").build())
        .await
        .expect("add bar domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with url output type
    test_jig.cmd.arg("foo.example").arg("-O").arg("url");

    // THEN output type is the urls
    let assert = test_jig.cmd.assert();
    assert.success().stdout(format!(
        "{}/domain/foo.example\n{}/domain/bar.example\n",
        test_jig.rdap_base, test_jig.rdap_base
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_referral_with_url_output_for_registry() {
    // GIVEN domain that refers to another domain (e.g. registry -> registrar)
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("foo.example")
            .link(
                Link::builder()
                    .rel("related")
                    // note that in real life this would be a foo.example referrign to a foo.example
                    // in another server. However, to get this to work with one server, we
                    // refer foo.example to bar.example.
                    .href(format!("{}/domain/bar.example", test_jig.rdap_base))
                    .value(format!("{}/domain/foo.example", test_jig.rdap_base))
                    .build(),
            )
            .build(),
    )
    .await
    .expect("add foo domain in tx");
    tx.add_domain(&Domain::builder().ldh_name("bar.example").build())
        .await
        .expect("add bar domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with url output type for registry
    test_jig
        .cmd
        .arg("foo.example")
        .arg("-O")
        .arg("url")
        .arg("-p")
        .arg("registry");

    // THEN output type is the urls
    let assert = test_jig.cmd.assert();
    assert
        .success()
        .stdout(format!("{}/domain/foo.example\n", test_jig.rdap_base));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_domain_referral_with_url_output_for_registrar() {
    // GIVEN domain that refers to another domain (e.g. registry -> registrar)
    let mut test_jig = TestJig::new_rdap().await;
    let mut tx = test_jig.mem.new_tx().await.expect("new transaction");
    tx.add_domain(
        &Domain::builder()
            .ldh_name("foo.example")
            .link(
                Link::builder()
                    .rel("related")
                    // note that in real life this would be a foo.example referrign to a foo.example
                    // in another server. However, to get this to work with one server, we
                    // refer foo.example to bar.example.
                    .href(format!("{}/domain/bar.example", test_jig.rdap_base))
                    .value(format!("{}/domain/foo.example", test_jig.rdap_base))
                    .build(),
            )
            .build(),
    )
    .await
    .expect("add foo domain in tx");
    tx.add_domain(&Domain::builder().ldh_name("bar.example").build())
        .await
        .expect("add bar domain in tx");
    tx.commit().await.expect("tx commit");

    // WHEN query with url output type for registrar
    test_jig
        .cmd
        .arg("foo.example")
        .arg("-O")
        .arg("url")
        .arg("-p")
        .arg("registrar");

    // THEN output type is the urls
    let assert = test_jig.cmd.assert();
    assert
        .success()
        .stdout(format!("{}/domain/bar.example\n", test_jig.rdap_base));
}
