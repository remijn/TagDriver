use enum_primitive::FromPrimitive;

enum_from_primitive! {

    #[derive(Debug, PartialEq)]
    pub enum NMDeviceState {
        Unknown = 0, //the device's state is unknown
        Unmanaged = 10, //the device is recognized, but not managed by NetworkManager
        Unavailable = 20, //the device is managed by NetworkManager, but is not available for use. Reasons may include the wireless switched off, missing firmware, no ethernet carrier, missing supplicant or modem manager, etc.
        Disconnected = 30, //the device can be activated, but is currently idle and not connected to a network.
        Prepare = 40, //the device is preparing the connection to the network. This may include operations like changing the MAC address, setting physical link properties, and anything else required to connect to the requested network.
        Config = 50, //the device is connecting to the requested network. This may include operations like associating with the WiFi AP, dialing the modem, connecting to the remote Bluetooth device, etc.
        NeedAuth = 60, //the device requires more information to continue connecting to the requested network. This includes secrets like WiFi passphrases, login passwords, PIN codes, etc.
        IpConfig = 70, //the device is requesting IPv4 and/or IPv6 addresses and routing information from the network.
        IpCheck = 80, //the device is checking whether further action is required for the requested network connection. This may include checking whether only local network access is available, whether a captive portal is blocking access to the Internet, etc.
        Secondaries = 90, //the device is waiting for a secondary connection (like a VPN) which must activated before the device can be activated
        Activated = 100, //the device has a network connection, either local or global.
        Deactivating = 110, //a disconnection from the current network connection was requested, and the device is cleaning up resources used for that connection. The network connection may still be valid.
        Failed = 120, //the device failed to connect to the requested network and is cleaning up the connection request
    }
}

impl NMDeviceState {
    pub fn from_int(int: u32) -> NMDeviceState {
        let Some(state) = NMDeviceState::from_u32(int) else {
            return NMDeviceState::Unknown;
        };
        state
    }
}
