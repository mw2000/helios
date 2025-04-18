use std::sync::Arc;

use eyre::Result;
use url::Url;

#[derive(Debug, Clone)]
pub enum ExecutionMode {
    Full(Arc<Vec<String>>),
    Verifiable(Arc<Vec<String>>, Arc<Url>),
}

impl ExecutionMode {
    pub fn from_urls(execution_rpcs: Option<Vec<String>>, execution_verifiable_api: Option<Url>) -> Self {
        match (execution_rpcs, execution_verifiable_api) {
            (Some(rpcs), Some(api)) => Self::Verifiable(Arc::new(rpcs), Arc::new(api)),
            (Some(rpcs), None) => Self::Full(Arc::new(rpcs)),
            (None, _) => Self::Full(Arc::new(vec![])),
        }
    }

    pub fn get_rpc(&self) -> Result<&str> {
        match self {
            Self::Full(rpcs) => {
                if rpcs.is_empty() {
                    return Err(eyre::eyre!("no execution RPCs provided"));
                }
                Ok(&rpcs[0])
            }
            Self::Verifiable(rpcs, _) => {
                if rpcs.is_empty() {
                    return Err(eyre::eyre!("no execution RPCs provided"));
                }
                Ok(&rpcs[0])
            }
        }
    }

    pub fn get_verifiable_api(&self) -> Option<&Url> {
        match self {
            Self::Verifiable(_, api) => Some(api),
            _ => None,
        }
    }

    pub fn get_all_rpcs(&self) -> &[String] {
        match self {
            Self::Full(rpcs) => rpcs,
            Self::Verifiable(rpcs, _) => rpcs,
        }
    }

    pub fn is_verifiable(&self) -> bool {
        matches!(self, Self::Verifiable(_, _))
    }
}
