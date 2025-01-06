// See https://www.usb.org/sites/default/files/documents/hut1_12v2.pdf

use num_derive::FromPrimitive;

#[derive(FromPrimitive, Debug)]
#[repr(u16)]
pub enum UsagePage {
    GenericDesktop = 0x01,
    SimulationsControl = 0x02,
    VrControls = 0x03,
    SportControls = 0x04,
    GameControls = 0x05,
    GenericDeviceControls = 0x06,
    KeyboardOrKeypad = 0x07,
    Led = 0x08,
    Button = 0x09,
    Ordinal = 0x0A,
    TelephonyDevice = 0x0C,
    Consumer = 0x0D,
    Digitizer = 0x0E,
    Unicode = 0x10,
    AlphanumericDisplay = 0x14,
    MedicalInstrument = 0x40,
}

#[derive(FromPrimitive, Debug)]
#[repr(u16)]
pub enum GenericDesktopUsage {
    Pointer = 0x01,
    Mouse = 0x02,
    // 0x03 is reserved
    Joystick = 0x04,
    GamePad = 0x05,
    Keyboard = 0x06,
    Keypad = 0x07,
    MultiAxisController = 0x08,
    TabletPcSystemControls = 0x09,
    // 0x0A-0x2F are reserved
    X = 0x30,
    Y = 0x31,
    Z = 0x32,
    Rx = 0x33,
    Ry = 0x34,
    Rz = 0x35,
    Slider = 0x36,
    Dial = 0x37,
    Wheel = 0x38,
    HatSwitch = 0x39,
    CountedBuffer = 0x3A,
    ByteCount = 0x3B,
    MotionWakeup = 0x3C,
    Start = 0x3D,
    Select = 0x3E,
    // 0x3F is reserved
    Vx = 0x40,
    Vy = 0x41,
    Vz = 0x42,
    Vbrx = 0x43,
    Vbry = 0x44,
    Vbrz = 0x45,
    Vno = 0x46,
    FeatureNotification = 0x47,
    ResolutionMultiplier = 0x48,
    // 0x49-0x7F are reserved
    SystemControl = 0x80,
    SystemPowerDown = 0x81,
    SystemSleep = 0x82,
    SystemWakeUp = 0x83,
    SystemContextMenu = 0x84,
    SystemMainMenu = 0x85,
    SystemAppMenu = 0x86,
    SystemMenuHelp = 0x87,
    SystemMenuExit = 0x88,
    SystemMenuSelect = 0x89,
    SystemMenuRight = 0x8A,
    SystemMenuLeft = 0x8B,
    SystemMenuUp = 0x8C,
    SystemMenuDown = 0x8D,
    SystemColdRestart = 0x8E,
    SystemWarmRestart = 0x8F,
    DpadUp = 0x90,
    DpadDown = 0x91,
    DpadRight = 0x92,
    DpadLeft = 0x93,
    // 0x94-0x9F are reserved
    //TODO: items past 0xA0
}

#[derive(FromPrimitive, Debug)]
#[repr(u8)]
pub enum KeyboardOrKeypadUsage {
    KbdErrorRollover = 0x1,
    KbdPostFail,
    KbdErrorUndefined,
    // the rest are used as regular keycodes
}
