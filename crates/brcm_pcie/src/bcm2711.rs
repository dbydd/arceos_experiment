use core::{marker::PhantomData, ptr::NonNull};

use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

use crate::BCM2711Hal;

register_bitfields![
    u32,
    // /* BRCM_PCIE_CAP_REGS - Offset for the mandatory capability config regs */
    // 0x00ac
    // BRCM_PCIE_CAP_REGS []，

    //  Broadcom STB PCIe Register Offsets
    // 0x0188
    RC_CFG_VENDOR_VENDOR_SPECIFIC_REG1 [
        LITTLE_ENDIAN OFFSET(0) NUMBITS(1) [],
        ENDIAN_MODE_BAR2 OFFSET(0xC) NUMBITS(1) [],
    ],

    // 0x043c
    RC_CFG_PRIV1_ID_VAL3 [
        CLASS_ID  OFFSET(0) NUMBITS(24) [
            pcie_pcie_bridge = 0x060400
        ],
    ],
    // 0x04dc
    // PCIE_RC_CFG_PRIV1_LINK_CAPABILITY [],


    // 0x1100
    // RC_DL_MDIO_ADDR [],

    // 0x1104
    // RC_DL_MDIO_WR_DATA [],
    // 0x1108
    // RC_DL_MDIO_RD_DATA

    // 0x4008
    MISC_MISC_CTRL [
        SCB_ACCESS_EN OFFSET(12) NUMBITS(1) [],
        CFG_READ_UR_MODE OFFSET(13) NUMBITS(1) [],
        MAX_BURST_SIZE OFFSET(20) NUMBITS(2) [],
        SCB0_SIZE OFFSET(27) NUMBITS(5) [
            init_val = 0x17,
        ],
        SCB1_SIZE OFFSET(22) NUMBITS(5) [],
        SCB2_SIZE OFFSET(0) NUMBITS(5) [],
    ],
    // 0x400c
    MISC_CPU_2_PCIE_MEM_WIN0_LO [
        MEM_WIN0_LO OFFSET(0) NUMBITS(32) [
            // TODO
            init_val = 0x0000_0000
        ],
    ],
    // 0x4010
    MISC_CPU_2_PCIE_MEM_WIN0_HI [
        MEM_WIN0_HI OFFSET(0) NUMBITS(32) [
            init_val = 0x0000_0006
        ],
    ],

    // 0x4204


    // 0x402C
    MISC_RC_BAR1_CONFIG_LO [
        MEM_WIN OFFSET(0) NUMBITS(5)[]
    ],

    // 0x4034
    MISC_RC_BAR2_CONFIG_LO [
        VALUE_LO OFFSET(0) NUMBITS(32)[
            init_val = 0x11,
        ]
    ],
    // 0x4038
    MISC_RC_BAR2_CONFIG_HI [
        VALUE_HI OFFSET(0) NUMBITS(32)[
            init_val = 0x4,
        ]
    ],
    // 0x403C
    MISC_RC_BAR3_CONFIG_LO [
        MEM_WIN OFFSET(0) NUMBITS(5)[]
    ],

    // 0x4044
    // MISC_MSI_BAR_CONFIG_LO
    // 0x4048
    // MISC_MSI_BAR_CONFIG_HI
    // 0x404c
    // MISC_MSI_DATA_CONFIG
    // 0x4060
    // MISC_EOI_CTRL
    // 0x4064
    // MISC_PCIE_CTRL
    // 0x4068
    MISC_PCIE_STATUS [
        CHECK_BITS OFFSET(4) NUMBITS(2)[],
        RC_MODE OFFSET(7) NUMBITS(1)[],
    ],
    // 0x406c
    MISC_REVISION [
        MISC_REVISION OFFSET(0) NUMBITS(32)[]
    ],

    // 0x4070
    MISC_CPU_2_PCIE_MEM_WIN0_BASE_LIMIT [
        MEM_WIN0_BASE_LIMIT OFFSET(0) NUMBITS(32)[
            // TODO
            init_val = 0
        ]
    ],
    // 0x4080
    MISC_CPU_2_PCIE_MEM_WIN0_BASE_HI [
        MEM_WIN0_BASE_HI OFFSET(0) NUMBITS(32)[
            init_val = 6
        ]
    ],
    // 0x4084
    MISC_CPU_2_PCIE_MEM_WIN0_LIMIT_HI [
        MEM_WIN0_LIMIT_HI OFFSET(0) NUMBITS(32)[
            init_val = 6
        ]
    ],
    // 0x4204
    MISC_HARD_PCIE_HARD_DEBUG [
        CLKREQ_DEBUG_ENABLE OFFSET(0) NUMBITS(1) [],
        CLKREQ_L1SS_ENABLE OFFSET(21) NUMBITS(1) [],
        SERDES_IDDQ OFFSET(27) NUMBITS(1) [],
    ],

    // 0x4300 INTR2_CPU_BASE
    INTR2_CPU_STATUS [
        INTR_STATUS OFFSET(0) NUMBITS(32) [],
    ],
    // 0x4304 0x4300 + 0x4
    INTR2_CPU_SET [
        INTR_SET OFFSET(0) NUMBITS(32) [],
    ],
    // 0x4308 0x4300 + 0x8
    INTR2_CPU_CLR [
        INTR_CLR OFFSET(0) NUMBITS(32) []
    ],
    // 0x430c 0x4300 + 0x0c
    INTR2_CPU_MASK_STATUS [
        INTR_MASK_STATUS OFFSET(0) NUMBITS(32) []
    ],
    // 0x4310 0x4300 + 0x10
    INTR2_CPU_MASK_SET [
        INTR_MASK_SET OFFSET(0) NUMBITS(32) []
    ],
    // 0x4314 0x4500 + 0x14
    INTR2_CPU_MASK_CLR [
        INTR_MASK_CLR OFFSET(0) NUMBITS(32) []
    ],
    // 0x4500 MSI_INTR2_BASE
    MSI_INTR2_STATUS [
        INTR_STATUS OFFSET(0) NUMBITS(32) [],
    ],
    // 0x4504 0x4500 + 0x4
    MSI_INTR2_SET [
        INTR_SET OFFSET(0) NUMBITS(32) [],
    ],
    // 0x4508 0x4500 + 0x8
    MSI_INTR2_CLR [
        INTR_CLR OFFSET(0) NUMBITS(32) []
    ],
    // 0x450c 0x4500 + 0x0c
    MSI_INTR2_MASK_STATUS [
        INTR_MASK_STATUS OFFSET(0) NUMBITS(32) []
    ],
    // 0x4510 0x4500 + 0x10
    MSI_INTR2_MASK_SET [
        INTR_MASK_SET OFFSET(0) NUMBITS(32) []
    ],
    // 0x4514 0x4500 + 0x14
    MSI_INTR2_MASK_CLR [
        INTR_MASK_CLR OFFSET(0) NUMBITS(32) []
    ],


    // 0x8000
    // EXT_CFG_DATA
    // 0x9000
    // EXT_CFG_INDEX

    // 0x9210
    RGR1_SW_INIT_1 [
        PCIE_RGR1_SW_INTI_1_PERST OFFSET(0) NUMBITS(1) [],
        RGR1_SW_INTI_1_GENERIC OFFSET(1) NUMBITS(1) [],
    ],

];

register_structs! {
    /// Pl011 registers.
    BCM2711PCIeHostBridgeRegs {
        (0x00 => _rsvd1),
        (0x0188 => rc_cfg_vendor_vendor_specific_reg1),
        (0x043c => rc_cfg_priv1_id_val3: ReadWrite<u32,RC_CFG_PRIV1_ID_VAL3::Register>),
        (0x0440 => _rsvdd2),
        (0x1100 => rc_dl_mdio_addr),
        (0x1104 => rc_dl_mdio_wr_data),
        (0x1108 => rc_dl_mdio_rd_data),
        (0x4008 => misc_misc_ctrl: ReadWrite<u32, MISC_MISC_CTRL::Register>),
        (0x400C => misc_cpu_2_pcie_mem_win0_lo: ReadWrite<u32,MISC_CPU_2_PCIE_MEM_WIN0_LO::Register>),
        (0x4010 => misc_cpu_2_pcie_mem_win0_hi: ReadWrite<u32,MISC_CPU_2_PCIE_MEM_WIN0_HI::Register>),
        (0x4014 => _rsvd22),
        (0x4028 => _rsvd2),
        (0x402C => misc_rc_bar1_config_lo: ReadWrite<u32,MISC_RC_BAR1_CONFIG_LO::Register>),
        (0x4030 => _rsvdd),
        (0x4034 => misc_rc_bar2_config_lo: ReadWrite<u32,MISC_RC_BAR2_CONFIG_LO::Register>),
        (0x4038 => misc_rc_bar2_config_hi: ReadWrite<u32,MISC_RC_BAR2_CONFIG_HI::Register>),
        (0x403C => misc_rc_bar3_config_lo: ReadWrite<u32,MISC_RC_BAR3_CONFIG_LO::Register>),
        (0x4040 => _rsvddd),
        (0x4044 => misc_msi_bar_config_lo),
        (0x4048 => misc_msi_bar_config_hi),
        (0x404c => misc_msi_data_config	),
        (0x4060 => misc_eoi_ctrl),
        (0x4064 => misc_pcie_ctrl),
        (0x4068 => misc_pcie_status: ReadOnly<u32,MISC_PCIE_STATUS::Register>),
        (0x406C => misc_revision: ReadWrite<u32,MISC_REVISION::Register>),
        (0x4070 => misc_cpu_2_pcie_mem_win0_base_limit: ReadWrite<u32, MISC_CPU_2_PCIE_MEM_WIN0_BASE_LIMIT::Register>),
        (0x4074 => hole),
        (0x4080 => misc_cpu_2_pcie_mem_win0_base_hi: ReadWrite<u32,MISC_CPU_2_PCIE_MEM_WIN0_BASE_HI::Register>),
        (0x4084 => misc_cpu_2_pcie_mem_win0_limit_hi: ReadWrite<u32,MISC_CPU_2_PCIE_MEM_WIN0_LIMIT_HI::Register>),
        (0x4088 => hole2),
        (0x4204 => misc_hard_pcie_hard_debug: ReadWrite<u32,MISC_HARD_PCIE_HARD_DEBUG::Register>),
        (0x4208 => _rsvd3),
        /// cpu intr
        (0x4300 => intr2_cpu_status:        ReadWrite<u32,INTR2_CPU_STATUS::Register>),
        (0x4304 => intr2_cpu_set:           ReadWrite<u32,INTR2_CPU_SET::Register>),
        (0x4308 => intr2_cpu_clr:           ReadWrite<u32,INTR2_CPU_CLR::Register>),
        (0x430C => intr2_cpu_mask_status:   ReadWrite<u32,INTR2_CPU_MASK_STATUS::Register>),
        (0x4310 => intr2_cpu_mask_set:      ReadWrite<u32,INTR2_CPU_MASK_SET::Register>),
        (0x4314 => intr2_cpu_mask_clr:      ReadWrite<u32,INTR2_CPU_MASK_CLR::Register>),
        (0x4318 => hole3),
        /// msi intr
        (0x4500 => msi_intr2_status:        ReadWrite<u32,MSI_INTR2_STATUS::Register>),
        (0x4504 => msi_intr2_set:           ReadWrite<u32,MSI_INTR2_SET::Register>),
        (0x4508 => msi_intr2_clr:           ReadWrite<u32,MSI_INTR2_CLR::Register>),
        (0x450C => msi_intr2_mask_status:   ReadWrite<u32,MSI_INTR2_MASK_STATUS::Register>),
        (0x4510 => msi_intr2_mask_set:      ReadWrite<u32,MSI_INTR2_MASK_SET::Register>),
        (0x4514 => msi_intr2_mask_clr:      ReadWrite<u32,MSI_INTR2_MASK_CLR::Register>),
        (0x4518 => hole4),
        /// Interrupt Clear Register.
        (0x9210 => rgr1_sw_init: ReadWrite<u32,RGR1_SW_INIT_1::Register>),
        (0x9214 => _rsvd4),
        (0x9310 => @END),
    }
}

impl BCM2711PCIeHostBridgeRegs {
    fn bridge_sw_init_set(&self, bit: u32) {
        if bit == 1 {
            self.rgr1_sw_init
                .modify(RGR1_SW_INIT_1::RGR1_SW_INTI_1_GENERIC::SET)
        }
        if bit == 0 {
            self.rgr1_sw_init
                .modify(RGR1_SW_INIT_1::RGR1_SW_INTI_1_GENERIC::CLEAR)
        }
    }

    fn perst_set(&self, bit: u32) {
        if bit == 1 {
            self.rgr1_sw_init
                .modify(RGR1_SW_INIT_1::PCIE_RGR1_SW_INTI_1_PERST::SET)
        }
        if bit == 0 {
            self.rgr1_sw_init
                .modify(RGR1_SW_INIT_1::PCIE_RGR1_SW_INTI_1_PERST::CLEAR)
        }
    }
}

pub struct BCM2711PCIeHostBridge<H: BCM2711Hal> {
    base: NonNull<BCM2711PCIeHostBridgeRegs>,
    _phantom: PhantomData<H>,
}

unsafe impl<H: BCM2711Hal> Send for BCM2711PCIeHostBridge<H> {}
unsafe impl<H: BCM2711Hal> Sync for BCM2711PCIeHostBridge<H> {}

impl<H: BCM2711Hal> BCM2711PCIeHostBridge<H> {
    /// Constrcut a new BCM2711 PCIe host bridge instance from the base address.
    pub const fn new(base: usize) -> Self {
        Self {
            base: NonNull::new(base as *mut u8).unwrap().cast(),
            _phantom: PhantomData,
        }
    }

    const fn regs(&self) -> &BCM2711PCIeHostBridgeRegs {
        unsafe { self.base.as_ref() }
    }

    pub fn setup(&self) {
        let regs = self.regs();

        // assert bridge reset
        regs.bridge_sw_init_set(1);
        log::debug!("assert bridge reset");

        // assert fundamental reset
        regs.perst_set(1);
        log::debug!("assert fundamental reset");

        H::sleep(core::time::Duration::from_micros(2 * 5));

        // deassert bridge reset
        regs.bridge_sw_init_set(0);
        log::debug!("deassert bridge reset");

        H::sleep(core::time::Duration::from_micros(2 * 5));

        // enable serdes
        regs.misc_hard_pcie_hard_debug
            .modify(MISC_HARD_PCIE_HARD_DEBUG::SERDES_IDDQ::CLEAR);
        log::debug!("enable serdes");

        H::sleep(core::time::Duration::from_micros(2 * 5));

        // get hardware revision
        let hw_rev = regs.misc_revision.read(MISC_REVISION::MISC_REVISION) & 0xFFFF;

        log::debug!("hw_rev{}", hw_rev);
        // disable and clear any pending interrupts
        regs.msi_intr2_clr.write(MSI_INTR2_CLR::INTR_CLR::SET);
        regs.msi_intr2_mask_set
            .write(MSI_INTR2_MASK_SET::INTR_MASK_SET::SET);

        log::debug!("disable and clear any pending interrupts");

        // Initialize set SCB_MAX_BURST_SIZE 0x0, CFG_READ_UR_MODE, SCB_ACCESS_EN
        regs.misc_misc_ctrl
            .modify(MISC_MISC_CTRL::SCB_ACCESS_EN::SET);
        regs.misc_misc_ctrl
            .modify(MISC_MISC_CTRL::CFG_READ_UR_MODE::SET);
        regs.misc_misc_ctrl
            .modify(MISC_MISC_CTRL::MAX_BURST_SIZE::CLEAR);

        // setup inbound memory view
        regs.misc_rc_bar2_config_lo
            .write(MISC_RC_BAR2_CONFIG_LO::VALUE_LO::init_val);
        regs.misc_rc_bar2_config_hi
            .write(MISC_RC_BAR2_CONFIG_HI::VALUE_HI::init_val);

        //
        regs.misc_misc_ctrl
            .modify(MISC_MISC_CTRL::SCB0_SIZE::init_val);

        // disable PCIe->GISB memory window and PCIe->SCB memory window
        regs.misc_rc_bar1_config_lo
            .modify(MISC_RC_BAR1_CONFIG_LO::MEM_WIN::CLEAR);
        regs.misc_rc_bar3_config_lo
            .modify(MISC_RC_BAR3_CONFIG_LO::MEM_WIN::CLEAR);

        // setup MSIs
        // clear interrupts
        // CPU::MMIOWrite32(pcieBase + MSI_BAR_CONFIG_LO, (MSI_TARGET_ADDR & 0xFFFFFFFFu) | 1);
        // mask interrupts
        // CPU::MMIOWrite32(pcieBase + MSI_BAR_CONFIG_HI, MSI_TARGET_ADDR >> 32);
        // CPU::MMIOWrite32(pcieBase + MSI_DATA_CONFIG, hwRev >= HW_REV_33 ? 0xffe06540 : 0xFFF86540);
        // TODO: add MSI handler registration here

        // cap controller to Gen2

        // deassert fundamental reset
        regs.perst_set(0);

        // wait for bits 4 and 5 of [0xfd504068] to be set, checking every 5000 us
        for _ in 0..20 {
            let val = regs.misc_pcie_status.read(MISC_PCIE_STATUS::CHECK_BITS);
            log::trace!("val :{}", val);
            if val == 0x3 {
                break;
            }
            H::sleep(core::time::Duration::from_micros(5 * 5));
        }

        // check if link is up
        loop {
            let val = regs.misc_pcie_status.read(MISC_PCIE_STATUS::CHECK_BITS);

            if val != 0x3 {
                // panic!("PCIe link is down");
                H::sleep(core::time::Duration::from_micros(20));
            } else {
                break;
            }
        }
        // log PCIe link is up

        // check if controller is running in root complex mode. if bit 7 is not set, and error
        {
            let val = regs.misc_pcie_status.read(MISC_PCIE_STATUS::RC_MODE);
            if val != 0x1 {
                panic!("PCIe controller is not running in root complex mode");
            }
        }

        log::debug!("PCIe link is ready");
        // log PCIe link is ready

        // outbound memory
        regs.misc_cpu_2_pcie_mem_win0_lo
            .write(MISC_CPU_2_PCIE_MEM_WIN0_LO::MEM_WIN0_LO::init_val);
        regs.misc_cpu_2_pcie_mem_win0_hi
            .write(MISC_CPU_2_PCIE_MEM_WIN0_HI::MEM_WIN0_HI::init_val);
        regs.misc_cpu_2_pcie_mem_win0_base_limit
            .write(MISC_CPU_2_PCIE_MEM_WIN0_BASE_LIMIT::MEM_WIN0_BASE_LIMIT::init_val);
        regs.misc_cpu_2_pcie_mem_win0_base_hi
            .write(MISC_CPU_2_PCIE_MEM_WIN0_BASE_HI::MEM_WIN0_BASE_HI::init_val);
        regs.misc_cpu_2_pcie_mem_win0_limit_hi
            .write(MISC_CPU_2_PCIE_MEM_WIN0_LIMIT_HI::MEM_WIN0_LIMIT_HI::init_val);

        // set proper class Id
        regs.rc_cfg_priv1_id_val3
            .modify(RC_CFG_PRIV1_ID_VAL3::CLASS_ID::pcie_pcie_bridge)

        // set proper endian
        // writeField(pcieBase + RC_CFG_VENDOR_VENDOR_SPECIFIC_REG1,
        //     RC_CFG_VENDOR_VENDOR_SPECIFIC_REG1_ENDIAN_MODE_BAR2_MASK,
        //     RC_CFG_VENDOR_VENDOR_SPECIFIC_REG1_ENDIAN_MODE_BAR2_SHIFT,
        //     DATA_ENDIAN);

        // set debug mode
        //     writeField(pcieBase + HARD_PCIE_HARD_DEBUG, HARD_PCIE_HARD_DEBUG_CLKREQ_DEBUG_ENABLE_MASK,
        // HARD_PCIE_HARD_DEBUG_CLKREQ_DEBUG_ENABLE_SHIFT, 1);
    }
}
