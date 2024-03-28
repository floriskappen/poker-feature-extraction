use cdrs_tokio::cluster::session::{TcpSessionBuilder, SessionBuilder, Session};
use cdrs_tokio::cluster::{NodeTcpConfigBuilder, QueryPager, SessionPager, TcpConnectionManager};
use cdrs_tokio::frame::TryFromRow;
use cdrs_tokio::load_balancing::RoundRobinLoadBalancingStrategy;
use cdrs_tokio::types::IntoRustByName;
use cdrs_tokio::{query::*, query_values};
use cdrs_tokio::transport::TransportTcp;
use cdrs_tokio::{IntoCdrsValue, TryFromRow};

use crate::constants::{DATABASE_KEYSPACE, DATABASE_TABLE, RETRIEVE_BATCH_SIZE};

pub type DatabaseSession = Session<TransportTcp, TcpConnectionManager, RoundRobinLoadBalancingStrategy<TransportTcp, TcpConnectionManager>>;
pub type DatabaseQueryPager<'a> = QueryPager<'a, String, SessionPager<'a, TransportTcp, TcpConnectionManager, RoundRobinLoadBalancingStrategy<TransportTcp, TcpConnectionManager>>>;
pub type DatabasePager<'a> = SessionPager<'a, TransportTcp, TcpConnectionManager, RoundRobinLoadBalancingStrategy<TransportTcp, TcpConnectionManager>>;

#[derive(Clone, Debug, IntoCdrsValue, TryFromRow, PartialEq)]
pub struct DatabasePokerHand {
    pub cards_id: String,
    pub histogram: Option<Vec<i8>>,
    pub token: Option<i64>
}

impl DatabasePokerHand {
    fn into_query_values(self) -> QueryValues {
        // **IMPORTANT NOTE:** query values should be WITHOUT NAMES
        // https://github.com/apache/cassandra/blob/trunk/doc/native_protocol_v4.spec#L413
        query_values!(self.histogram, self.cards_id)
    }
}

pub async fn create_session() -> DatabaseSession {
    let cluster_config = NodeTcpConfigBuilder::new()
        .with_contact_point("127.0.0.1:9042".into())
        .build()
        .await
        .unwrap();
    return TcpSessionBuilder::new(RoundRobinLoadBalancingStrategy::new(), cluster_config)
        .build()
        .await
        .unwrap();
}

pub async fn retrieve_batch(
    session: &DatabaseSession,
    last_token_value: Option<i64>
) -> Vec<DatabasePokerHand> {
    let query: String;
    if let Some(token_value) = last_token_value {
        query = format!("SELECT cards_id, histogram, token(cards_id) FROM {}.{} WHERE token(cards_id) > {} LIMIT {};", DATABASE_KEYSPACE, DATABASE_TABLE, token_value, RETRIEVE_BATCH_SIZE);
    } else {
        query = format!("SELECT cards_id, histogram, token(cards_id) FROM {}.{} LIMIT {};", DATABASE_KEYSPACE, DATABASE_TABLE, RETRIEVE_BATCH_SIZE);
    }

    let rows = session.query(query)
        .await
        .expect("query")
        .response_body()
        .expect("get body")
        .into_rows()
        .expect("into rows");

    let result: Vec<DatabasePokerHand> = rows.iter()
        .map(|row| {
            let cards_id: String = row.get_by_name("cards_id").expect("read cards_id").unwrap();
            
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
        println!("GOT BATCH: {:?}", result[result.len()-1].token);
    } else {
        println!("RESULTS EMPTY!! LAST BATCH DONE")
    }
    return result;
}

pub async fn update_batch(
    session: &DatabaseSession,
    hands: Vec<DatabasePokerHand>,
) {
    let mut batch = BatchQueryBuilder::new();
    for hand in hands {
        let query = format!("UPDATE {}.{} SET histogram = ? WHERE cards_id = ?", DATABASE_KEYSPACE, DATABASE_TABLE);
        batch = batch.add_query(query, hand.into_query_values());
    }

    let batch_query = batch.build().expect("Batch builder");
    session.batch(batch_query).await.expect("Batch query error");
}
