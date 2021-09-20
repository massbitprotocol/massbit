use anyhow::Error;
use async_trait::async_trait;
use futures03::Stream;

use super::{Block, BlockPtr, Blockchain};
use crate::components::store::BlockNumber;

pub trait BlockStream<C: Blockchain>:
    Stream<Item = Result<BlockStreamEvent<C>, Error>> + Unpin
{
}

pub struct BlockWithTriggers<C: Blockchain> {
    pub block: C::Block,
    pub trigger_data: Vec<C::TriggerData>,
}

impl<C: Blockchain> BlockWithTriggers<C> {
    pub fn new(block: C::Block, mut trigger_data: Vec<C::TriggerData>) -> Self {
        trigger_data.sort();
        Self {
            block,
            trigger_data,
        }
    }

    pub fn trigger_count(&self) -> usize {
        self.trigger_data.len()
    }

    pub fn ptr(&self) -> BlockPtr {
        self.block.ptr()
    }
}

#[async_trait]
pub trait TriggersAdapter<C: Blockchain>: Send + Sync {
    // Returns a sequence of blocks in increasing order of block number.
    // Each block will include all of its triggers that match the given `filter`.
    // The sequence may omit blocks that contain no triggers,
    // but all returned blocks must part of a same chain starting at `chain_base`.
    // At least one block will be returned, even if it contains no triggers.
    // `step_size` is the suggested number blocks to be scanned.
    async fn scan_triggers(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        filter: &C::TriggerFilter,
    ) -> Result<Vec<BlockWithTriggers<C>>, Error>;
}

pub enum BlockStreamEvent<C: Blockchain> {
    ProcessBlock(BlockWithTriggers<C>),
}
