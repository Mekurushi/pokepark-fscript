use cranelift_entity::PrimaryMap;
use fsc_mir::{BasicBlockId, Body};

pub(super) struct BlockMapping {
    targets: PrimaryMap<BasicBlockId, fsb_ir::BlockId>,
}

impl BlockMapping {
    pub(super) fn allocate(body: &Body, output: &mut fsb_ir::Function) -> Self {
        let mut targets = PrimaryMap::with_capacity(body.blocks.len());

        for source in body.blocks.keys() {
            let allocated = targets.push(output.create_block());
            debug_assert_eq!(allocated, source);
        }

        Self { targets }
    }

    pub(super) fn target(&self, source: BasicBlockId) -> fsb_ir::BlockId {
        self.targets[source]
    }
}
