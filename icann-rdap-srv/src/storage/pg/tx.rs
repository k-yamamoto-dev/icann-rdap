#![allow(clippy::diverging_sub_expression)]
use {
    async_trait::async_trait,
    icann_rdap_common::response::{Autnum, Domain, Entity, Nameserver, Network, Rfc9083Error},
    sqlx::{PgPool, Postgres},
};

use crate::{
    error::RdapServerError,
    storage::{
        data::{AutnumId, DomainId, EntityId, NameserverId, NetworkId},
        TxHandle,
    },
};

pub struct PgTx<'a> {
    db_tx: sqlx::Transaction<'a, Postgres>,
}

impl PgTx<'_> {
    pub async fn new(pg_pool: &PgPool) -> Result<Self, RdapServerError> {
        let db_tx = pg_pool.begin().await?;
        Ok(Self { db_tx })
    }

    pub async fn new_truncate(pg_pool: &PgPool) -> Result<Self, RdapServerError> {
        let mut db_tx = pg_pool.begin().await?;
        // TODO actually complete this
        // this is just here to make sure something will compile
        sqlx::query("truncate domain").execute(&mut *db_tx).await?;
        Ok(Self { db_tx })
    }
}

#[async_trait]
impl TxHandle for PgTx<'_> {
    async fn add_entity(&mut self, _entity: &Entity) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn add_entity_err(
        &mut self,
        _entity_id: &EntityId,
        _error: &Rfc9083Error,
    ) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn add_domain(&mut self, _domain: &Domain) -> Result<(), RdapServerError> {
        // TODO actually complete this
        // this is just here to make sure something will compile
        sqlx::query("insert domain")
            .execute(&mut *self.db_tx)
            .await?;
        Ok(())
    }

    async fn add_domain_err(
        &mut self,
        _domain_id: &DomainId,
        _error: &Rfc9083Error,
    ) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn add_nameserver(&mut self, _nameserver: &Nameserver) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn add_nameserver_err(
        &mut self,
        _nameserver_id: &NameserverId,
        _error: &Rfc9083Error,
    ) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn add_autnum(&mut self, _autnum: &Autnum) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn add_autnum_err(
        &mut self,
        _autnum_id: &AutnumId,
        _error: &Rfc9083Error,
    ) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn add_network(&mut self, _network: &Network) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn add_network_err(
        &mut self,
        _network_id: &NetworkId,
        _error: &Rfc9083Error,
    ) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn add_srv_help(
        &mut self,
        _help: &icann_rdap_common::response::Help,
        _host: Option<&str>,
    ) -> Result<(), RdapServerError> {
        todo!()
    }

    async fn commit(self: Box<Self>) -> Result<(), RdapServerError> {
        self.db_tx.commit().await?;
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> Result<(), RdapServerError> {
        self.db_tx.rollback().await?;
        Ok(())
    }
}
