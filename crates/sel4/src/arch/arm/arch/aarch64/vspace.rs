use crate::{
    cap_type, sys, CapType, FrameType, IPCBuffer, ObjectBlueprint, ObjectBlueprintAArch64,
    ObjectBlueprintArm,
};

/// Frame sizes for AArch64.
#[derive(Copy, Clone, Debug)]
pub enum FrameSize {
    Small,
    Large,
    Huge,
}

impl FrameSize {
    pub const fn blueprint(self) -> ObjectBlueprint {
        match self {
            Self::Small => ObjectBlueprintArm::SmallPage.into(),
            Self::Large => ObjectBlueprintArm::LargePage.into(),
            Self::Huge => ObjectBlueprintAArch64::HugePage.into(),
        }
    }

    // For match arm LHS's, as we can't call const fn's
    pub const SMALL_BITS: usize = Self::Small.bits();
    pub const LARGE_BITS: usize = Self::Large.bits();
    pub const HUGE_BITS: usize = Self::Huge.bits();
}

impl FrameType for cap_type::SmallPage {
    const FRAME_SIZE: FrameSize = FrameSize::Small;
}

impl FrameType for cap_type::LargePage {
    const FRAME_SIZE: FrameSize = FrameSize::Large;
}

impl FrameType for cap_type::HugePage {
    const FRAME_SIZE: FrameSize = FrameSize::Huge;
}

//

const LEVEL_BITS: usize = 9;

pub trait TranslationTableType: CapType {
    const SPAN_BITS: usize;
    const SPAN_BYTES: usize = 1 << Self::SPAN_BITS;

    fn _map_raw(
        ipc_buffer: &mut IPCBuffer,
        service: sys::seL4_CPtr,
        vspace: sys::seL4_CPtr,
        vaddr: sys::seL4_Word,
        attr: sys::seL4_ARM_VMAttributes::Type,
    ) -> sys::seL4_Error::Type;
}

impl TranslationTableType for cap_type::PUD {
    const SPAN_BITS: usize = cap_type::PD::SPAN_BITS + LEVEL_BITS;

    fn _map_raw(
        ipc_buffer: &mut IPCBuffer,
        service: sys::seL4_CPtr,
        vspace: sys::seL4_CPtr,
        vaddr: sys::seL4_Word,
        attr: sys::seL4_ARM_VMAttributes::Type,
    ) -> sys::seL4_Error::Type {
        ipc_buffer
            .inner_mut()
            .seL4_ARM_PageUpperDirectory_Map(service, vspace, vaddr, attr)
    }
}

impl TranslationTableType for cap_type::PD {
    const SPAN_BITS: usize = cap_type::PT::SPAN_BITS + LEVEL_BITS;

    fn _map_raw(
        ipc_buffer: &mut IPCBuffer,
        service: sys::seL4_CPtr,
        vspace: sys::seL4_CPtr,
        vaddr: sys::seL4_Word,
        attr: sys::seL4_ARM_VMAttributes::Type,
    ) -> sys::seL4_Error::Type {
        ipc_buffer
            .inner_mut()
            .seL4_ARM_PageDirectory_Map(service, vspace, vaddr, attr)
    }
}

impl TranslationTableType for cap_type::PT {
    const SPAN_BITS: usize = FrameSize::Small.bits() + LEVEL_BITS;

    fn _map_raw(
        ipc_buffer: &mut IPCBuffer,
        service: sys::seL4_CPtr,
        vspace: sys::seL4_CPtr,
        vaddr: sys::seL4_Word,
        attr: sys::seL4_ARM_VMAttributes::Type,
    ) -> sys::seL4_Error::Type {
        ipc_buffer
            .inner_mut()
            .seL4_ARM_PageTable_Map(service, vspace, vaddr, attr)
    }
}
