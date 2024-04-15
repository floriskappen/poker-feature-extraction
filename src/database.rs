use std::sync::Arc;
use std::error::Error;
use std::env;
use std::time::Duration;

use cdrs_tokio::authenticators::StaticPasswordAuthenticatorProvider;
use cdrs_tokio::cluster::session::{TcpSessionBuilder, SessionBuilder, Session};
use cdrs_tokio::cluster::{NodeTcpConfigBuilder, QueryPager, SessionPager, TcpConnectionManager};
use cdrs_tokio::load_balancing::RoundRobinLoadBalancingStrategy;
use cdrs_tokio::types::blob::Blob;
use cdrs_tokio::types::IntoRustByName;
use cdrs_tokio::{query::*, query_values};
use cdrs_tokio::transport::TransportTcp;
use cdrs_tokio::{IntoCdrsValue, TryFromRow};
use anyhow::{Result as AnyhowResult, Context};

use crate::constants::{DATABASE_KEYSPACE, DATABASE_TABLE, RETRIEVE_BATCH_SIZE};

pub type DatabaseSession = Session<TransportTcp, TcpConnectionManager, RoundRobinLoadBalancingStrategy<TransportTcp, TcpConnectionManager>>;
pub type DatabaseQueryPager<'a> = QueryPager<'a, String, SessionPager<'a, TransportTcp, TcpConnectionManager, RoundRobinLoadBalancingStrategy<TransportTcp, TcpConnectionManager>>>;
pub type DatabasePager<'a> = SessionPager<'a, TransportTcp, TcpConnectionManager, RoundRobinLoadBalancingStrategy<TransportTcp, TcpConnectionManager>>;

#[derive(Clone, Debug, IntoCdrsValue, TryFromRow, PartialEq)]
pub struct DatabasePokerHand {
    pub cards_id: i64,
    pub histogram: Option<Blob>,
    pub token: Option<i64>
}

impl DatabasePokerHand {
    fn into_query_values(self) -> QueryValues {
        // **IMPORTANT NOTE:** query values should be WITHOUT NAMES
        // https://github.com/apache/cassandra/blob/trunk/doc/native_protocol_v4.spec#L413
        query_values!(self.histogram, self.cards_id)
    }
}

pub async fn create_session() -> AnyhowResult<DatabaseSession> {
    let authenticator = Arc::new(StaticPasswordAuthenticatorProvider::new(
        env::var("DATABASE_USERNAME").unwrap(),
        env::var("DATABASE_PASSWORD").unwrap(),
    ));
    let cluster_config = NodeTcpConfigBuilder::new()
        .with_contact_point(format!("{}:{}", env::var("DATABASE_HOST").unwrap(), env::var("DATABASE_PORT").unwrap()).into())
        .with_authenticator_provider(authenticator)
        .build()
        .await?;
    let session = TcpSessionBuilder::new(RoundRobinLoadBalancingStrategy::new(), cluster_config)
        .build()
        .await?;

    return Ok(session)
}

pub async fn create_session_with_retry() -> DatabaseSession {
    loop {
        match create_session().await {
            Ok(session) => return session,
            Err(err) => {
                // Handle session creation error
                eprintln!("Session creation error: {:?}", err);
                println!("Retrying session creation after 5s...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

pub async fn retrieve_batch(
    session: &DatabaseSession,
    last_token_value: Option<i64>
) -> Result<Vec<DatabasePokerHand>, cdrs_tokio::types::prelude::Error> {
    let query: String;
    if let Some(token_value) = last_token_value {
        query = format!("SELECT cards_id, token(cards_id) FROM {}.{} WHERE token(cards_id) > {} LIMIT {};", DATABASE_KEYSPACE, DATABASE_TABLE, token_value, RETRIEVE_BATCH_SIZE);
    } else {
        query = format!("SELECT cards_id, token(cards_id) FROM {}.{} LIMIT {};", DATABASE_KEYSPACE, DATABASE_TABLE, RETRIEVE_BATCH_SIZE);
    }

    let rows = session.query(query)
        .await?
        .response_body()
        .expect("get body")
        .into_rows()
        .expect("into rows");

    let result: Vec<DatabasePokerHand> = rows.iter()
        .map(|row| {
            let cards_id: i64 = row.get_by_name("cards_id").expect("read cards_id").unwrap();

            // Manually extract the token using the column name as it appears in the query result
            let token: Option<i64> = row.get_by_name("system.token(cards_id)").expect("");

            DatabasePokerHand {
                cards_id,
                histogram: None,
                token,
            }
        })
        .collect();

    if result.len() > 0 {
        println!("Retrieved batch for token {:?} to {:?}", result[0].token, result[result.len()-1].token);
    } else {
        println!("RESULTS EMPTY!! LAST BATCH DONE")
    }
    return Ok(result);
}

pub async fn update_batch(
    session: &DatabaseSession,
    hands: Vec<DatabasePokerHand>,
) -> AnyhowResult<()> {
    let mut batch = BatchQueryBuilder::new();
    for hand in hands {
        let query = format!("UPDATE {}.{} SET histogram = ? WHERE cards_id = ?", DATABASE_KEYSPACE, DATABASE_TABLE);
        batch = batch.add_query(query, hand.into_query_values());
    }

    let batch_query = batch.build().expect("Batch builder");
    session.batch(batch_query).await?;

    Ok(())
}
