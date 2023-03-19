use std::sync::Arc;

use ethers::prelude::*;

/// Subscribe to the rpc endpoint "SubscribePending"
pub async fn subscribe_pending_txs_with_body(
    client: &Arc<Provider<Ws>>,
) -> Result<SubscriptionStream<'_, Ws, Transaction>, ProviderError>
{
    // this rpc is erigon specific
    client.subscribe(["newPendingTransactionsWithBody"]).await
}
