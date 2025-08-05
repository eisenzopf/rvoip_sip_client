use std::net::{IpAddr, Ipv4Addr};
use local_ip_address::{list_afinet_netifas, local_ip};

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: IpAddr,
    pub display_name: String,
}

impl NetworkInterface {
    pub fn new(name: String, ip: IpAddr) -> Self {
        let display_name = format!("{} ({})", Self::friendly_name(&name), ip);
        Self { name, ip, display_name }
    }
    
    fn friendly_name(name: &str) -> &str {
        // Make interface names more user-friendly
        match name {
            n if n.starts_with("en") && n.len() <= 4 => "Ethernet",
            n if n.starts_with("eth") => "Ethernet",
            n if n.starts_with("wl") => "Wi-Fi",
            n if n.starts_with("wi") => "Wi-Fi",
            "lo" | "lo0" => "Loopback",
            n if n.contains("docker") => "Docker",
            n if n.contains("vmnet") => "VMware",
            n if n.contains("vbox") => "VirtualBox",
            n if n.contains("bridge") => "Bridge",
            n if n.contains("tap") => "TAP",
            n if n.contains("tun") => "TUN",
            _ => name,
        }
    }
}

pub fn get_available_interfaces() -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();
    
    if let Ok(network_interfaces) = list_afinet_netifas() {
        for (name, ip) in network_interfaces {
            // Filter out IPv6 for now, and loopback
            if let IpAddr::V4(ipv4) = ip {
                // Skip loopback unless it's the only option
                if !ipv4.is_loopback() || interfaces.is_empty() {
                    interfaces.push(NetworkInterface::new(name, ip));
                }
            }
        }
    }
    
    // If no interfaces found, add localhost as fallback
    if interfaces.is_empty() {
        interfaces.push(NetworkInterface::new(
            "lo".to_string(),
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        ));
    }
    
    // Sort interfaces to put more likely choices first
    interfaces.sort_by(|a, b| {
        // Prioritize non-loopback, then ethernet, then wifi
        let a_priority = match (a.ip.is_loopback(), NetworkInterface::friendly_name(&a.name)) {
            (true, _) => 3,
            (false, "Ethernet") => 0,
            (false, "Wi-Fi") => 1,
            (false, _) => 2,
        };
        let b_priority = match (b.ip.is_loopback(), NetworkInterface::friendly_name(&b.name)) {
            (true, _) => 3,
            (false, "Ethernet") => 0,
            (false, "Wi-Fi") => 1,
            (false, _) => 2,
        };
        a_priority.cmp(&b_priority)
    });
    
    interfaces
}

pub fn get_default_interface() -> Option<IpAddr> {
    // Try to get the default local IP
    local_ip().ok()
}