//! Network interface information for the `system.netInterfaces` capability.
//!
//! Implemented with the Windows IP Helper API so the data is
//! locale-independent (no parsing of localized `ipconfig` output).

use serde::Serialize;

/// One adapter as seen by the host.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterface {
    /// Friendly name, e.g. "WLAN".
    pub name: String,
    /// Adapter description, e.g. "Intel(R) Wi-Fi 6E AX211".
    pub description: String,
    /// MAC address, uppercase hex with colons.
    pub mac: String,
    /// "up" / "down" / "unknown" …
    pub status: String,
    /// IPv4 addresses in dotted form.
    pub ipv4: Vec<String>,
    /// IPv6 addresses in compressed form.
    pub ipv6: Vec<String>,
    /// Prefix length of the first IPv4 address.
    pub prefix_length: Option<u8>,
    /// DNS servers of the adapter.
    pub dns_servers: Vec<String>,
    /// Default gateway, if the adapter has one.
    pub gateway: Option<String>,
    pub gateways: Vec<String>,
    pub dns_suffix: String,
    pub dhcp_enabled: bool,
    pub dhcp_v4_server: Option<String>,
    pub dhcp_v6_server: Option<String>,
    pub mtu: u32,
    pub interface_index: u32,
    pub interface_type: u32,
    pub transmit_link_speed: u64,
    pub receive_link_speed: u64,
    pub ipv4_metric: u32,
    pub ipv6_metric: u32,
    pub addresses: Vec<InterfaceAddress>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceAddress {
    pub address: String,
    pub prefix_length: u8,
    pub subnet_mask: Option<String>,
}

pub fn subnet_mask(prefix: u8) -> Option<String> {
    if prefix > 32 { return None; }
    Some(std::net::Ipv4Addr::from(u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0)).to_string())
}

/// List the adapters of the machine.
pub fn list_interfaces() -> Vec<NetworkInterface> {
    #[cfg(windows)]
    {
        windows::list_interfaces()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

pub fn configuration_report() -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::Globalization::{GetOEMCP, MultiByteToWideChar};
        let system = std::env::var_os("SystemRoot").ok_or("无法定位 Windows 系统目录")?;
        let output = std::process::Command::new(std::path::PathBuf::from(system).join("System32/ipconfig.exe"))
            .arg("/all").creation_flags(0x08000000).output().map_err(|e| e.to_string())?;
        if !output.status.success() { return Err("ipconfig /all 执行失败".into()); }
        let bytes = output.stdout;
        if bytes.len() > 2 * 1024 * 1024 { return Err("网络报告超出大小限制".into()); }
        let count = unsafe { MultiByteToWideChar(GetOEMCP(), 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0) };
        if count <= 0 { return Err("网络报告解码失败".into()); }
        let mut text = vec![0u16; count as usize];
        unsafe { MultiByteToWideChar(GetOEMCP(), 0, bytes.as_ptr(), bytes.len() as i32, text.as_mut_ptr(), count); }
        Ok(String::from_utf16_lossy(&text))
    }
    #[cfg(not(windows))]
    { Err("仅支持 Windows".into()) }
}

/// Format a MAC address the way `ipconfig` does.
pub fn format_mac(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Format four address bytes as a dotted IPv4 address.
pub fn format_ipv4(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 4 {
        return None;
    }
    Some(format!(
        "{}.{}.{}.{}",
        bytes[0], bytes[1], bytes[2], bytes[3]
    ))
}

/// Format a network-order IPv4 value (as Windows stores it) as text.
pub fn format_ipv4_u32(raw: u32) -> String {
    format_ipv4(&raw.to_ne_bytes()).unwrap_or_default()
}

/// Format sixteen address bytes as an IPv6 address.
pub fn format_ipv6(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 16 {
        return None;
    }
    let groups: Vec<String> = bytes
        .chunks(2)
        .map(|pair| format!("{:x}", u16::from_be_bytes([pair[0], pair[1]])))
        .collect();
    Some(groups.join(":"))
}

/// Map an `IF_OPER_STATUS` value onto a readable label.
pub fn status_label(status: u32) -> &'static str {
    match status {
        1 => "up",
        2 => "down",
        3 => "testing",
        4 => "unknown",
        5 => "dormant",
        6 => "not present",
        _ => "lower layer down",
    }
}

#[cfg(windows)]
mod windows {
    use super::{format_ipv4_u32, format_ipv6, format_mac, status_label, subnet_mask, InterfaceAddress, NetworkInterface};
    use windows_sys::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GetIpForwardTable, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_MULTICAST,
        IP_ADAPTER_ADDRESSES_LH, MIB_IPFORWARDROW, MIB_IPFORWARDTABLE, GAA_FLAG_INCLUDE_GATEWAYS,
    };
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, SOCKET_ADDRESS,
    };

    const NO_ERROR: u32 = 0;

    /// Read a wide string that the API allocated for us.
    unsafe fn wide_string(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }

    /// Read the IPv4 / IPv6 address out of a socket address.
    unsafe fn socket_address(addr: &SOCKET_ADDRESS) -> Option<String> {
        if addr.lpSockaddr.is_null() {
            return None;
        }
        let sa: &SOCKADDR = &*addr.lpSockaddr;
        match sa.sa_family {
            AF_INET => {
                let sin: &SOCKADDR_IN = &*(addr.lpSockaddr as *const SOCKADDR_IN);
                let raw = unsafe { sin.sin_addr.S_un.S_addr };
                // The address is stored in network order; read the raw
                // memory order back out.
                Some(format_ipv4_u32(raw))
            }
            AF_INET6 => {
                let sin: &SOCKADDR_IN6 = &*(addr.lpSockaddr as *const SOCKADDR_IN6);
                let bytes = unsafe { sin.sin6_addr.u.Byte };
                let address = format_ipv6(&bytes)?;
                let scope = unsafe { sin.Anonymous.sin6_scope_id };
                Some(if scope == 0 { address } else { format!("{address}%{scope}") })
            }
            _ => None,
        }
    }

    pub(super) fn list_interfaces() -> Vec<NetworkInterface> {
        let gateways = gateway_map();

        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_INCLUDE_GATEWAYS;
        let mut size: u32 = 15_000;
        let mut buffer: Vec<u64> = Vec::new();

        let result = loop {
            buffer.clear();
            buffer.resize((size as usize).div_ceil(8) + 1, 0);
            let mut required = size;
            let ret = unsafe {
                GetAdaptersAddresses(
                    AF_UNSPEC as u32,
                    flags,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                    &mut required,
                )
            };
            if ret == ERROR_BUFFER_OVERFLOW {
                size = required.max(size + 4096);
                continue;
            }
            break ret;
        };
        if result != NO_ERROR {
            return Vec::new();
        }

        let mut out = Vec::new();
        let mut cursor = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        while !cursor.is_null() {
            let adapter = unsafe { &*cursor };

            let name = unsafe { wide_string(adapter.FriendlyName) };
            let description = unsafe { wide_string(adapter.Description) };
            let mac =
                format_mac(&adapter.PhysicalAddress[..adapter.PhysicalAddressLength as usize]);

            let mut ipv4 = Vec::new();
            let mut ipv6 = Vec::new();
            let mut prefix_length = None;
            let mut addresses = Vec::new();
            let mut unicast = adapter.FirstUnicastAddress;
            while !unicast.is_null() {
                let entry = unsafe { &*unicast };
                if let Some(address) = unsafe { socket_address(&entry.Address) } {
                    addresses.push(InterfaceAddress { address: address.clone(), prefix_length: entry.OnLinkPrefixLength, subnet_mask: if address.contains(':') { None } else { subnet_mask(entry.OnLinkPrefixLength) } });
                    if address.contains(':') {
                        ipv6.push(address);
                    } else {
                        if prefix_length.is_none() {
                            prefix_length = Some(entry.OnLinkPrefixLength);
                        }
                        ipv4.push(address);
                    }
                }
                unicast = entry.Next;
            }

            let mut dns_servers = Vec::new();
            let mut dns = adapter.FirstDnsServerAddress;
            while !dns.is_null() {
                let entry = unsafe { &*dns };
                if let Some(address) = unsafe { socket_address(&entry.Address) } {
                    if !dns_servers.contains(&address) {
                        dns_servers.push(address);
                    }
                }
                dns = entry.Next;
            }

            let if_index = unsafe { adapter.Anonymous1.Anonymous.IfIndex };
            let mut all_gateways = Vec::new();
            let mut entry = adapter.FirstGatewayAddress;
            while !entry.is_null() {
                let current = unsafe { &*entry };
                if let Some(address) = unsafe { socket_address(&current.Address) } { all_gateways.push(address); }
                entry = current.Next;
            }
            if all_gateways.is_empty() { all_gateways.extend(gateways
                .iter()
                .filter(|(index, _)| *index == if_index)
                .map(|(_, address)| address.clone())); }
            let gateway = all_gateways.first().cloned();

            out.push(NetworkInterface {
                name,
                description,
                mac,
                status: status_label(adapter.OperStatus as u32).to_string(),
                ipv4,
                ipv6,
                prefix_length,
                dns_servers,
                gateway,
                gateways: all_gateways,
                dns_suffix: unsafe { wide_string(adapter.DnsSuffix) },
                dhcp_enabled: unsafe { adapter.Anonymous2.Flags & 4 != 0 },
                dhcp_v4_server: unsafe { socket_address(&adapter.Dhcpv4Server) },
                dhcp_v6_server: unsafe { socket_address(&adapter.Dhcpv6Server) },
                mtu: adapter.Mtu,
                interface_index: if_index,
                interface_type: adapter.IfType,
                transmit_link_speed: adapter.TransmitLinkSpeed,
                receive_link_speed: adapter.ReceiveLinkSpeed,
                ipv4_metric: adapter.Ipv4Metric,
                ipv6_metric: adapter.Ipv6Metric,
                addresses,
            });

            cursor = adapter.Next;
        }
        out
    }

    /// Default gateway per adapter interface index (from the IPv4 route
    /// table: the default route is the one with destination 0.0.0.0).
    fn gateway_map() -> Vec<(u32, String)> {
        let mut size: u32 = 0;
        unsafe {
            GetIpForwardTable(std::ptr::null_mut(), &mut size, 0);
        }
        if size == 0 {
            return Vec::new();
        }
        let mut buffer = vec![0u8; size as usize];
        let ret = unsafe {
            GetIpForwardTable(buffer.as_mut_ptr() as *mut MIB_IPFORWARDTABLE, &mut size, 0)
        };
        if ret != NO_ERROR {
            return Vec::new();
        }

        // MIB_IPFORWARDTABLE { DWORD dwNumEntries; MIB_IPFORWARDROW Table[]; }
        let entries = u32::from_ne_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
        let row_size = std::mem::size_of::<MIB_IPFORWARDROW>();
        let mut out = Vec::new();
        for index in 0..entries {
            let offset = 4 + index * row_size;
            if offset + row_size > buffer.len() {
                break;
            }
            let read_u32 = |field: usize| -> u32 {
                let start = offset + field;
                u32::from_ne_bytes([
                    buffer[start],
                    buffer[start + 1],
                    buffer[start + 2],
                    buffer[start + 3],
                ])
            };
            // dest(0) mask(4) policy(8) nextHop(12) ifIndex(16)
            let dest = read_u32(0);
            let mask = read_u32(4);
            if dest != 0 || mask != 0 {
                continue;
            }
            let next_hop = read_u32(12);
            let if_index = read_u32(16);
            // The next hop is stored in network order.
            let gateway = format_ipv4_u32(next_hop);
            if gateway == "0.0.0.0" {
                continue;
            }
            out.push((if_index, gateway));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subnet_masks_cover_boundaries_and_invalid_prefix() {
        assert_eq!(subnet_mask(0).as_deref(), Some("0.0.0.0"));
        assert_eq!(subnet_mask(24).as_deref(), Some("255.255.255.0"));
        assert_eq!(subnet_mask(32).as_deref(), Some("255.255.255.255"));
        assert_eq!(subnet_mask(33), None);
    }

    #[test]
    fn formats_mac_addresses() {
        assert_eq!(
            format_mac(&[0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E]),
            "00:1A:2B:3C:4D:5E"
        );
        assert_eq!(format_mac(&[]), "");
    }

    #[test]
    fn formats_ipv4_addresses() {
        assert_eq!(
            format_ipv4(&[192, 168, 1, 23]).as_deref(),
            Some("192.168.1.23")
        );
        assert_eq!(format_ipv4(&[1, 2, 3]), None);
    }

    #[test]
    fn formats_network_order_ipv4_values() {
        // 192.168.5.2 stored in network order reads back as 0x0205A8C0.
        assert_eq!(format_ipv4_u32(0x0205A8C0), "192.168.5.2");
        assert_eq!(format_ipv4_u32(0), "0.0.0.0");
    }

    #[test]
    fn formats_ipv6_addresses() {
        let bytes: Vec<u8> = (0..16).collect();
        assert_eq!(
            format_ipv6(&bytes).as_deref(),
            Some("1:203:405:607:809:a0b:c0d:e0f")
        );
        assert_eq!(format_ipv6(&[0; 4]), None);
    }

    #[test]
    fn maps_oper_status_labels() {
        assert_eq!(status_label(1), "up");
        assert_eq!(status_label(2), "down");
        assert_eq!(status_label(4), "unknown");
    }

    #[test]
    fn listing_never_panics() {
        // The exact result depends on the machine; only the shape matters.
        for interface in list_interfaces() {
            assert!(!interface.name.is_empty() || !interface.description.is_empty());
        }
    }
}
