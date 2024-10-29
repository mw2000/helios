use std::{fmt::Display, net::SocketAddr, sync::Arc};

use alloy::network::{ReceiptResponse, TransactionResponse};
use alloy::primitives::{Address, Bytes, B256, U256, U64};
use alloy::rpc::json_rpc::RpcObject;
use alloy::rpc::types::{Filter, Log, SyncStatus};
use eyre::Result;
use jsonrpsee::{
    core::{async_trait, server::Methods},
    proc_macros::rpc,
    server::{ServerBuilder, ServerHandle},
    types::error::{ErrorObject, ErrorObjectOwned},
};
use tracing::info;

use crate::client::node::Node;
use crate::consensus::Consensus;
use crate::network_spec::NetworkSpec;
use crate::types::{Block, BlockTag};

pub struct Rpc<N: NetworkSpec, C: Consensus<N::TransactionResponse>> {
    node: Arc<Node<N, C>>,
    handle: Option<ServerHandle>,
    address: SocketAddr,
}

impl<N: NetworkSpec, C: Consensus<N::TransactionResponse>> Rpc<N, C> {
    pub fn new(node: Arc<Node<N, C>>, address: SocketAddr) -> Self {
        Rpc {
            node,
            handle: None,
            address,
        }
    }

    pub async fn start(&mut self) -> Result<SocketAddr> {
        let rpc_inner = RpcInner {
            node: self.node.clone(),
            address: self.address,
        };

        let (handle, addr) = start(rpc_inner).await?;
        self.handle = Some(handle);

        info!(target: "helios::rpc", "rpc server started at {}", addr);

        Ok(addr)
    }
}

#[rpc(server, namespace = "eth")]
trait EthRpc<TX: TransactionResponse + RpcObject, TXR: RpcObject, R: ReceiptResponse + RpcObject> {
    #[method(name = "getBalance")]
    async fn get_balance(
        &self,
        address: Address,
        block: BlockTag,
    ) -> Result<U256, ErrorObjectOwned>;
    #[method(name = "getTransactionCount")]
    async fn get_transaction_count(
        &self,
        address: Address,
        block: BlockTag,
    ) -> Result<U64, ErrorObjectOwned>;
    #[method(name = "getBlockTransactionCountByHash")]
    async fn get_block_transaction_count_by_hash(
        &self,
        hash: B256,
    ) -> Result<U64, ErrorObjectOwned>;
    #[method(name = "getBlockTransactionCountByNumber")]
    async fn get_block_transaction_count_by_number(
        &self,
        block: BlockTag,
    ) -> Result<U64, ErrorObjectOwned>;
    #[method(name = "getCode")]
    async fn get_code(&self, address: Address, block: BlockTag) -> Result<Bytes, ErrorObjectOwned>;
    #[method(name = "getClientVersion")]
    async fn get_client_version(&self) -> Result<String, ErrorObjectOwned>;
    #[method(name = "call")]
    async fn call(&self, tx: TXR, block: BlockTag) -> Result<Bytes, ErrorObjectOwned>;
    #[method(name = "estimateGas")]
    async fn estimate_gas(&self, tx: TXR) -> Result<U64, ErrorObjectOwned>;
    #[method(name = "chainId")]
    async fn chain_id(&self) -> Result<U64, ErrorObjectOwned>;
    #[method(name = "gasPrice")]
    async fn gas_price(&self) -> Result<U256, ErrorObjectOwned>;
    #[method(name = "maxPriorityFeePerGas")]
    async fn max_priority_fee_per_gas(&self) -> Result<U256, ErrorObjectOwned>;
    #[method(name = "blockNumber")]
    async fn block_number(&self) -> Result<U64, ErrorObjectOwned>;
    #[method(name = "getBlockByNumber")]
    async fn get_block_by_number(
        &self,
        block: BlockTag,
        full_tx: bool,
    ) -> Result<Option<Block<TX>>, ErrorObjectOwned>;
    #[method(name = "getBlockByHash")]
    async fn get_block_by_hash(
        &self,
        hash: B256,
        full_tx: bool,
    ) -> Result<Option<Block<TX>>, ErrorObjectOwned>;
    #[method(name = "sendRawTransaction")]
    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<B256, ErrorObjectOwned>;
    #[method(name = "getTransactionReceipt")]
    async fn get_transaction_receipt(&self, hash: B256) -> Result<Option<R>, ErrorObjectOwned>;
    #[method(name = "getTransactionByHash")]
    async fn get_transaction_by_hash(&self, hash: B256) -> Result<Option<TX>, ErrorObjectOwned>;
    #[method(name = "getTransactionByBlockHashAndIndex")]
    async fn get_transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: U64,
    ) -> Result<Option<TX>, ErrorObjectOwned>;
    #[method(name = "getLogs")]
    async fn get_logs(&self, filter: Filter) -> Result<Vec<Log>, ErrorObjectOwned>;
    #[method(name = "getFilterChanges")]
    async fn get_filter_changes(&self, filter_id: U256) -> Result<Vec<Log>, ErrorObjectOwned>;
    #[method(name = "uninstallFilter")]
    async fn uninstall_filter(&self, filter_id: U256) -> Result<bool, ErrorObjectOwned>;
    #[method(name = "getNewFilter")]
    async fn get_new_filter(&self, filter: Filter) -> Result<U256, ErrorObjectOwned>;
    #[method(name = "getNewBlockFilter")]
    async fn get_new_block_filter(&self) -> Result<U256, ErrorObjectOwned>;
    #[method(name = "getNewPendingTransactionFilter")]
    async fn get_new_pending_transaction_filter(&self) -> Result<U256, ErrorObjectOwned>;
    #[method(name = "getStorageAt")]
    async fn get_storage_at(
        &self,
        address: Address,
        slot: B256,
        block: BlockTag,
    ) -> Result<U256, ErrorObjectOwned>;
    #[method(name = "coinbase")]
    async fn coinbase(&self) -> Result<Address, ErrorObjectOwned>;
    #[method(name = "syncing")]
    async fn syncing(&self) -> Result<SyncStatus, ErrorObjectOwned>;
}

#[rpc(client, server, namespace = "net")]
trait NetRpc {
    #[method(name = "version")]
    async fn version(&self) -> Result<u64, ErrorObjectOwned>;
}

struct RpcInner<N: NetworkSpec, C: Consensus<N::TransactionResponse>> {
    node: Arc<Node<N, C>>,
    address: SocketAddr,
}

impl<N: NetworkSpec, C: Consensus<N::TransactionResponse>> Clone for RpcInner<N, C> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            address: self.address,
        }
    }
}

#[async_trait]
impl<N: NetworkSpec, C: Consensus<N::TransactionResponse>>
    EthRpcServer<N::TransactionResponse, N::TransactionRequest, N::ReceiptResponse>
    for RpcInner<N, C>
{
    async fn get_balance(
        &self,
        address: Address,
        block: BlockTag,
    ) -> Result<U256, ErrorObjectOwned> {
        convert_err(self.node.get_balance(address, block).await)
    }

    async fn get_transaction_count(
        &self,
        address: Address,
        block: BlockTag,
    ) -> Result<U64, ErrorObjectOwned> {
        convert_err(self.node.get_nonce(address, block).await).map(U64::from)
    }

    async fn get_block_transaction_count_by_hash(
        &self,
        hash: B256,
    ) -> Result<U64, ErrorObjectOwned> {
        convert_err(self.node.get_block_transaction_count_by_hash(hash).await).map(U64::from)
    }

    async fn get_block_transaction_count_by_number(
        &self,
        block: BlockTag,
    ) -> Result<U64, ErrorObjectOwned> {
        convert_err(self.node.get_block_transaction_count_by_number(block).await).map(U64::from)
    }

    async fn get_code(&self, address: Address, block: BlockTag) -> Result<Bytes, ErrorObjectOwned> {
        convert_err(self.node.get_code(address, block).await)
    }

    async fn get_client_version(&self) -> Result<String, ErrorObjectOwned> {
        convert_err(self.node.get_client_version().await)
    }

    async fn call(
        &self,
        tx: N::TransactionRequest,
        block: BlockTag,
    ) -> Result<Bytes, ErrorObjectOwned> {
        convert_err(self.node.call(&tx, block).await)
    }

    async fn estimate_gas(&self, tx: N::TransactionRequest) -> Result<U64, ErrorObjectOwned> {
        let res = self.node.estimate_gas(&tx).await.map(U64::from);

        convert_err(res)
    }

    async fn chain_id(&self) -> Result<U64, ErrorObjectOwned> {
        Ok(U64::from(self.node.chain_id()))
    }

    async fn gas_price(&self) -> Result<U256, ErrorObjectOwned> {
        convert_err(self.node.get_gas_price().await)
    }

    async fn max_priority_fee_per_gas(&self) -> Result<U256, ErrorObjectOwned> {
        convert_err(self.node.get_priority_fee())
    }

    async fn block_number(&self) -> Result<U64, ErrorObjectOwned> {
        convert_err(self.node.get_block_number().await).map(U64::from)
    }

    async fn get_block_by_number(
        &self,
        block: BlockTag,
        full_tx: bool,
    ) -> Result<Option<Block<N::TransactionResponse>>, ErrorObjectOwned> {
        convert_err(self.node.get_block_by_number(block, full_tx).await)
    }

    async fn get_block_by_hash(
        &self,
        hash: B256,
        full_tx: bool,
    ) -> Result<Option<Block<N::TransactionResponse>>, ErrorObjectOwned> {
        convert_err(self.node.get_block_by_hash(hash, full_tx).await)
    }

    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<B256, ErrorObjectOwned> {
        convert_err(self.node.send_raw_transaction(&bytes).await)
    }

    async fn get_transaction_receipt(
        &self,
        hash: B256,
    ) -> Result<Option<N::ReceiptResponse>, ErrorObjectOwned> {
        convert_err(self.node.get_transaction_receipt(hash).await)
    }

    async fn get_transaction_by_hash(
        &self,
        hash: B256,
    ) -> Result<Option<N::TransactionResponse>, ErrorObjectOwned> {
        Ok(self.node.get_transaction_by_hash(hash).await)
    }

    async fn get_transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: U64,
    ) -> Result<Option<N::TransactionResponse>, ErrorObjectOwned> {
        Ok(self
            .node
            .get_transaction_by_block_hash_and_index(hash, index.to())
            .await)
    }

    async fn coinbase(&self) -> Result<Address, ErrorObjectOwned> {
        convert_err(self.node.get_coinbase().await)
    }

    async fn syncing(&self) -> Result<SyncStatus, ErrorObjectOwned> {
        convert_err(self.node.syncing().await)
    }

    async fn get_logs(&self, filter: Filter) -> Result<Vec<Log>, ErrorObjectOwned> {
        convert_err(self.node.get_logs(&filter).await)
    }

    async fn get_filter_changes(&self, filter_id: U256) -> Result<Vec<Log>, ErrorObjectOwned> {
        convert_err(self.node.get_filter_changes(filter_id).await)
    }

    async fn uninstall_filter(&self, filter_id: U256) -> Result<bool, ErrorObjectOwned> {
        convert_err(self.node.uninstall_filter(filter_id).await)
    }

    async fn get_new_filter(&self, filter: Filter) -> Result<U256, ErrorObjectOwned> {
        convert_err(self.node.get_new_filter(&filter).await)
    }

    async fn get_new_block_filter(&self) -> Result<U256, ErrorObjectOwned> {
        convert_err(self.node.get_new_block_filter().await)
    }

    async fn get_new_pending_transaction_filter(&self) -> Result<U256, ErrorObjectOwned> {
        convert_err(self.node.get_new_pending_transaction_filter().await)
    }

    async fn get_storage_at(
        &self,
        address: Address,
        slot: B256,
        block: BlockTag,
    ) -> Result<U256, ErrorObjectOwned> {
        convert_err(self.node.get_storage_at(address, slot, block).await)
    }
}

#[async_trait]
impl<N: NetworkSpec, C: Consensus<N::TransactionResponse>> NetRpcServer for RpcInner<N, C> {
    async fn version(&self) -> Result<u64, ErrorObjectOwned> {
        Ok(self.node.chain_id())
    }
}

async fn start<N: NetworkSpec, C: Consensus<N::TransactionResponse>>(
    rpc: RpcInner<N, C>,
) -> Result<(ServerHandle, SocketAddr)> {
    let server = ServerBuilder::default().build(rpc.address).await?;
    let addr = server.local_addr()?;

    let mut methods = Methods::new();
    let eth_methods: Methods = EthRpcServer::into_rpc(rpc.clone()).into();
    let net_methods: Methods = NetRpcServer::into_rpc(rpc).into();

    methods.merge(eth_methods)?;
    methods.merge(net_methods)?;

    let handle = server.start(methods);

    Ok((handle, addr))
}

fn convert_err<T, E: Display>(res: Result<T, E>) -> Result<T, ErrorObjectOwned> {
    res.map_err(|err| ErrorObject::owned(1, err.to_string(), None::<()>))
}
