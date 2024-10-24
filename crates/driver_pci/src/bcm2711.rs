use crate::types::{ConfigCommand, ConifgPciPciBridge};
use crate::{Access, PciAddress};
// use aarch64_cpu::registers::*;
use ratio::Ratio;
use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};

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
        SCB2_SIZE OFFSET(0) NUMBITS(1) [],
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
        RGR1_SW_INTI_1_PERST OFFSET(0) NUMBITS(1) [],
        RGR1_SW_INTI_1_INIT OFFSET(1) NUMBITS(1) [],
    ],

    RC_CFG_VENDOR_SPECIFIC_REG1 [
        ENDIAN_MODE OFFSET(2) NUMBITS(2) [
            LITTLE_ENDIAN = 0
        ],
    ],

    RC_CFG_PRIV1_LINK_CAPABILITY [
        ASPM_SUPPORT OFFSET(10) NUMBITS(2)[

        ]
    ]

];

register_structs! {
    /// Pl011 registers.
    BCM2711PCIeHostBridgeRegs {
        (0x00 => _rsvd1),
        (0x0188 => rc_cfg_vendor_vendor_specific_reg1: ReadWrite<u32,RC_CFG_VENDOR_SPECIFIC_REG1::Register>),
        (0x018C => _rsvd222),
        (0x043c => rc_cfg_priv1_id_val3: ReadWrite<u32,RC_CFG_PRIV1_ID_VAL3::Register>),
        (0x0440 => _rsvdd2),
        (0x04dc => rc_cfg_priv1_link_capability: ReadWrite<u32,RC_CFG_PRIV1_LINK_CAPABILITY::Register>),
        (0x04e0 => _rsvdd3),
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
    fn pcie_link_up(&self) -> bool {
        self.misc_pcie_status.read(MISC_PCIE_STATUS::CHECK_BITS) == 0x3
    }

    fn set_gen(&self, gen: u32) {
        const PCI_EXP_LNKCTL2: usize = 48;
        const PCI_EXP_LNKCAP: usize = 12;
        const PCI_EXP_LNKCAP_SLS: u32 = 0x0000000f;

        unsafe {
            let cap_base = self as *const BCM2711PCIeHostBridgeRegs as *const u8 as usize + 0x00ac;
            let mut lnkctl2 = ((cap_base + PCI_EXP_LNKCTL2) as *const u16).read_volatile();
            let mut lnkcap = ((cap_base + PCI_EXP_LNKCAP) as *const u32).read_volatile();
            lnkcap = (lnkcap & !PCI_EXP_LNKCAP_SLS) | gen;

            ((cap_base + PCI_EXP_LNKCAP) as *mut u32).write_volatile(lnkcap);

            lnkctl2 = (lnkctl2 & !0xf) | gen as u16;

            ((cap_base + PCI_EXP_LNKCTL2) as *mut u16).write_volatile(lnkctl2);
        }
    }
}

use core::time::Duration;
use log::{debug, info};

const RGR1_SW_INIT_1: usize = 0x9210;
const EXT_CFG_INDEX: usize = 0x9000;
const EXT_CFG_DATA: usize = 0x8000;
// const EXT_CFG_DATA: usize = 0x9004;

#[derive(Clone)]
pub struct BCM2711 {}

fn cfg_index(addr: PciAddress) -> usize {
    ((addr.device as u32) << 15 | (addr.function as u32) << 12 | (addr.bus as u32) << 20) as usize
}

impl BCM2711 {
    unsafe fn do_setup(mmio_base: usize) {
        // debug!(
        //     "PCIe 7link start @0x{:X}... value {:X}",
        //     mmio_base,
        //     *(mmio_base as *mut u32)
        // );

        debug!(
            "PCIe 7link start @0x{:X}...",
            mmio_base,
        );

        let regs = &mut *(mmio_base as *mut BCM2711PCIeHostBridgeRegs);
        debug!("\nreaded reg!");

        /*
         * Reset the bridge, assert the fundamental reset. Note for some SoCs,
         * e.g. BCM7278, the fundamental reset should not be asserted here.
         * This will need to be changed when support for other SoCs is added.
         */
        debug!("init!");
        regs.rgr1_sw_init.modify(
            RGR1_SW_INIT_1::RGR1_SW_INTI_1_INIT::SET + RGR1_SW_INIT_1::RGR1_SW_INTI_1_PERST::SET,
        );

        debug!("assert fundamental reset");
        /*
         * The delay is a safety precaution to preclude the reset signal
         * from looking like a glitch.
         */
        busy_wait(Duration::from_micros(100));

        /* Take the bridge out of reset */
        regs.rgr1_sw_init
            .modify(RGR1_SW_INIT_1::RGR1_SW_INTI_1_INIT::CLEAR);
        debug!("deassert bridge reset");

        // enable serdes
        regs.misc_hard_pcie_hard_debug
            .modify(MISC_HARD_PCIE_HARD_DEBUG::SERDES_IDDQ::CLEAR);
        debug!("enable serdes");
        /* Wait for SerDes to be stable */
        busy_wait(core::time::Duration::from_micros(100));

        /* Set SCB_MAX_BURST_SIZE, CFG_READ_UR_MODE, SCB_ACCESS_EN */
        regs.misc_misc_ctrl.write(
            MISC_MISC_CTRL::MAX_BURST_SIZE::SET
                + MISC_MISC_CTRL::SCB_ACCESS_EN::SET
                + MISC_MISC_CTRL::CFG_READ_UR_MODE::SET
                + MISC_MISC_CTRL::SCB2_SIZE::SET,
        );

        regs.misc_rc_bar2_config_lo
            .write(MISC_RC_BAR2_CONFIG_LO::VALUE_LO::init_val);
        regs.misc_rc_bar2_config_hi
            .write(MISC_RC_BAR2_CONFIG_HI::VALUE_HI::init_val);

        regs.misc_misc_ctrl.set(0x88003000);

        /* Disable the PCIe->GISB memory window (RC_BAR1) */
        regs.misc_rc_bar1_config_lo
            .modify(MISC_RC_BAR1_CONFIG_LO::MEM_WIN::CLEAR);
        /* Disable the PCIe->SCB memory window (RC_BAR3) */
        regs.misc_rc_bar3_config_lo
            .modify(MISC_RC_BAR3_CONFIG_LO::MEM_WIN::CLEAR);

        /* Mask all interrupts since we are not handling any yet */
        regs.msi_intr2_mask_set
            .write(MSI_INTR2_MASK_SET::INTR_MASK_SET::SET);

        /* Clear any interrupts we find on boot */
        regs.msi_intr2_clr.write(MSI_INTR2_CLR::INTR_CLR::SET);

        /* Unassert the fundamental reset */
        regs.rgr1_sw_init
            .modify(RGR1_SW_INIT_1::RGR1_SW_INTI_1_PERST::CLEAR);

        /*
         * Wait for 100ms after PERST# deassertion; see PCIe CEM specification
         * sections 2.2, PCIe r5.0, 6.6.1.
         */
        busy_wait(core::time::Duration::from_millis(100));

        debug!("PCIe wait for link up...");

        /* Give the RC/EP time to wake up, before trying to configure RC.
         * Intermittently check status for link-up, up to a total of 100ms.
         */
        for _ in 0..100 {
            if regs.pcie_link_up() {
                break;
            }
            busy_wait(core::time::Duration::from_millis(1));
        }

        if !regs.pcie_link_up() {
            panic!("pcie link down!");
        }

        // check if controller is running in root complex mode. if bit 7 is not set, and error
        {
            let val = regs.misc_pcie_status.read(MISC_PCIE_STATUS::RC_MODE);
            if val != 0x1 {
                panic!("PCIe controller is not running in root complex mode");
            }
        }

        // outbound memory
        // regs.misc_cpu_2_pcie_mem_win0_lo.set(0xC0000000);
        // regs.misc_cpu_2_pcie_mem_win0_hi.set(0x0);
        regs.misc_cpu_2_pcie_mem_win0_lo.set(0x0);
        regs.misc_cpu_2_pcie_mem_win0_hi.set(0x6);
        regs.misc_cpu_2_pcie_mem_win0_base_limit.set(0x3FF00000);
        regs.misc_cpu_2_pcie_mem_win0_base_hi
            .write(MISC_CPU_2_PCIE_MEM_WIN0_BASE_HI::MEM_WIN0_BASE_HI::init_val);
        regs.misc_cpu_2_pcie_mem_win0_limit_hi
            .write(MISC_CPU_2_PCIE_MEM_WIN0_LIMIT_HI::MEM_WIN0_LIMIT_HI::init_val);
        /*
         * For config space accesses on the RC, show the right class for
         * a PCIe-PCIe bridge (the default setting is to be EP mode).
         */
        regs.rc_cfg_priv1_id_val3
            .modify(RC_CFG_PRIV1_ID_VAL3::CLASS_ID::pcie_pcie_bridge);

        // ssc
        //todo

        let lnksta = ((mmio_base + 0xac + 18) as *const u16).read_volatile();

        let cls = lnksta & 0xf;

        let nlw = (lnksta & 0x03f0) >> 4;

        info!(
            "PCIe BRCM: link up, {} Gbps x{:?}",
            link_speed_to_str(cls),
            nlw
        );

        regs.rc_cfg_vendor_vendor_specific_reg1
            .write(RC_CFG_VENDOR_SPECIFIC_REG1::ENDIAN_MODE::LITTLE_ENDIAN);

        /*
         * We used to enable the CLKREQ# input here, but a few PCIe cards don't
         * attach anything to the CLKREQ# line, so we shouldn't assume that
         * it's connected and working. The controller does allow detecting
         * whether the port on the other side of our link is/was driving this
         * signal, so we could check before we assume. But because this signal
         * is for power management, which doesn't make sense in a bootloader,
         * let's instead just unadvertise ASPM support.
         */
        regs.rc_cfg_priv1_link_capability
            .modify(RC_CFG_PRIV1_LINK_CAPABILITY::ASPM_SUPPORT::CLEAR);
    }
}

fn link_speed_to_str(cls: u16) -> &'static str {
    match cls {
        0x1 => "2.5",
        0x2 => "5.0",
        0x3 => "8.0",
        _ => "??",
    }
}

impl Access for BCM2711 {
    fn map_conf(mmio_base: usize, addr: PciAddress) -> Option<usize> {
        // debug!("mmio_base {:X} addr {addr}", mmio_base);
        // bus 0 bus 1 只有一个Device
        // if addr.bus <= 2 && addr.device > 0 {
        //     return None;
        // }

        // if addr.bus == 0 {
        //     return Some(mmio_base);
        // }

        let cfg_address = mmio_base
            + ((addr.bus as usize) << 20)
            + ((addr.device as usize) << 15)
            + ((addr.function as usize) << 12);
        return Some(cfg_address);

        let idx = cfg_index(addr);
        unsafe {
            ((mmio_base + EXT_CFG_INDEX) as *mut u32).write_volatile(idx as u32);
        }
        return Some(mmio_base + EXT_CFG_DATA);
    }

    fn probe_bridge(mmio_base: usize, bridge: &ConifgPciPciBridge) {
        debug!("bridge bcm2711");

        bridge.set_cache_line_size(64 / 4);

        bridge.set_memory_base((0xF8000000u32 >> 16) as u16);
        bridge.set_memory_limit((0xF8000000u32 >> 16) as u16);
        bridge.set_control(0x01);
        unsafe {
            (bridge.cfg_addr as *mut u8)
                .offset(0xac + 0x1c)
                .write_volatile(0x10);
        }

        bridge.to_header().set_command([
            ConfigCommand::MemorySpaceEnable,
            ConfigCommand::BusMasterEnable,
            ConfigCommand::ParityErrorResponse,
            ConfigCommand::SERREnable,
        ])
    }

    fn setup(mmio_base: usize) {
        init_early();
        unsafe {
            Self::do_setup(mmio_base);
        }
    }
}

/// Number of nanoseconds in a second.
pub const NANOS_PER_SEC: u64 = 1_000_000_000;
static mut CNTPCT_TO_NANOS_RATIO: Ratio = Ratio::zero();
static mut NANOS_TO_CNTPCT_RATIO: Ratio = Ratio::zero();
/// Early stage initialization: stores the timer frequency.
fn init_early() {
    let freq = 
    {   
        #[cfg(platform="aarch64")]
        {
            use aarch64_cpu::registers::*;
            return CNTFRQ_EL0.get();
        }
        10_000_000
    };
    unsafe {
        CNTPCT_TO_NANOS_RATIO = Ratio::new(NANOS_PER_SEC as u32, freq as u32);
        NANOS_TO_CNTPCT_RATIO = CNTPCT_TO_NANOS_RATIO.inverse();
    }
}

pub fn current_ticks() -> u64 {

        #[cfg(arch="aarch64")]
        {
            use aarch64_cpu::registers::*;
            return CNTPCT_EL0.get()
        }
        0
}
/// Converts hardware ticks to nanoseconds.
#[inline]
pub fn ticks_to_nanos(ticks: u64) -> u64 {
    unsafe { CNTPCT_TO_NANOS_RATIO.mul_trunc(ticks) }
}

/// Converts nanoseconds to hardware ticks.
#[inline]
pub fn nanos_to_ticks(nanos: u64) -> u64 {
    unsafe { NANOS_TO_CNTPCT_RATIO.mul_trunc(nanos) }
}

/// Returns the current clock time in nanoseconds.
pub fn current_time_nanos() -> u64 {
    ticks_to_nanos(current_ticks())
}

pub fn current_time() -> Duration {
    Duration::from_nanos(current_time_nanos())
}

/// Busy waiting for the given duration.
pub fn busy_wait(dur: Duration) {
    busy_wait_until(current_time() + dur);
}

/// Busy waiting until reaching the given deadline.
pub fn busy_wait_until(deadline: Duration) {
    while current_time() < deadline {
        core::hint::spin_loop();
    }
}
