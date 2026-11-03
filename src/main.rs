use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, anyhow};
use bacnet_types::primitives::ObjectIdentifier;
use bacnetio::connection::{self, BACNET_PORT, Connection, Endpoint};
use bacnetio::diagnosis;
use bacnetio::interfaces::{self, Adapter, Subnet};
use bacnetio::points::{self, DeviceReading};
use bacnetio::scan::{self, Device, ScanReport};
use bacnetio::{export, present, server, storage, table};
use clap::{Args, Parser, Subcommand};

mod write_cli;

#[derive(Parser)]
#[command(
    version,
    about = "BACnet browser. Without a command it opens in the browser."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Port of the local web server
    #[arg(long, default_value_t = server::DEFAULT_PORT)]
    web_port: u16,
    /// Do not open the browser
    #[arg(long)]
    no_browser: bool,
    /// Folder for the write log (default: the user's local application data)
    #[arg(long, value_name = "DIR", global = true)]
    data_dir: Option<PathBuf>,
    /// Offer the loopback interface as an adapter, for the browser tests' simulated devices
    #[arg(long, hide = true)]
    test_loopback: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Find BACnet devices on the network
    Scan {
        #[command(flatten)]
        network: NetworkArgs,
        /// Do not ask each address of the subnet directly
        #[arg(long)]
        no_sweep: bool,
        /// Save the device list as CSV (for Excel)
        #[arg(long, value_name = "FILE")]
        csv: Option<PathBuf>,
        /// Only devices with an ID in this range, e.g. 1000-1999
        #[arg(long, value_name = "FROM-TO", value_parser = parse_range)]
        range: Option<(u32, u32)>,
        /// Ask every address of this subnet instead of the adapter's, e.g. a plant behind a VPN
        #[arg(long, value_name = "NET/BITS", value_parser = Subnet::parse)]
        subnet: Option<Subnet>,
    },
    /// Read the data points of one device
    Read {
        /// Device ID
        #[arg(required_unless_present = "address")]
        device: Option<u32>,
        /// IP address of the device; needed when it does not answer Who-Is
        #[arg(long)]
        address: Option<Ipv4Addr>,
        /// Save the point list as CSV (for Excel)
        #[arg(long, value_name = "FILE")]
        csv: Option<PathBuf>,
        #[command(flatten)]
        network: NetworkArgs,
    },
    /// Write a present value, e.g.: write 1001 ao:1 42.5 --priority 8
    Write {
        #[command(flatten)]
        target: DeviceArgs,
        /// Object: ai ao av bi bo bv msi mso msv and the instance, e.g. ao:1
        #[arg(value_parser = present::parse_object)]
        object: ObjectIdentifier,
        /// New value: a number, active/inactive, or a state number
        value: String,
        /// Priority 1 to 16; leave out for objects without priority array
        #[arg(long)]
        priority: Option<u8>,
    },
    /// Release a priority written before, e.g.: release 1001 ao:1 --priority 8
    Release {
        #[command(flatten)]
        target: DeviceArgs,
        #[arg(value_parser = present::parse_object)]
        object: ObjectIdentifier,
        #[arg(long)]
        priority: u8,
    },
    /// List network adapters
    Interfaces,
}

/// A device by ID, or by address when it does not answer Who-Is.
#[derive(Args)]
struct DeviceArgs {
    /// Device ID
    device: u32,
    /// IP address of the device; needed when it does not answer Who-Is
    #[arg(long)]
    address: Option<Ipv4Addr>,
    #[command(flatten)]
    network: NetworkArgs,
}

#[derive(Args)]
struct NetworkArgs {
    /// IP address of the network adapter to use (default: picked automatically)
    #[arg(long)]
    interface: Option<Ipv4Addr>,
    /// UDP port of the BACnet network
    #[arg(long, default_value_t = BACNET_PORT)]
    port: u16,
    /// Seconds to wait for answers to Who-Is
    #[arg(long, default_value_t = scan::DEFAULT_WHO_IS_WAIT.as_secs())]
    wait: u64,
    /// BBMD to register with, to reach devices in other subnets (IP or IP:PORT)
    #[arg(long, value_name = "ADDRESS", value_parser = connection::parse_address)]
    bbmd: Option<SocketAddrV4>,
}

fn parse_range(text: &str) -> Result<(u32, u32), String> {
    let invalid = || format!("{text} is not a range like 1000-1999");
    let (from, to) = text.split_once('-').ok_or_else(invalid)?;
    let from = from.trim().parse().map_err(|_| invalid())?;
    let to = to.trim().parse().map_err(|_| invalid())?;
    if from > to {
        return Err(invalid());
    }
    Ok((from, to))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let data_dir = cli.data_dir.or_else(storage::default_dir);
    let Some(command) = cli.command else {
        let served =
            server::serve(cli.web_port, !cli.no_browser, data_dir, cli.test_loopback).await;
        if let Err(error) = &served {
            eprintln!("Error: {error:#}");
            wait_for_enter();
        }
        return served;
    };
    match command {
        Command::Scan {
            network,
            no_sweep,
            csv,
            range,
            subnet,
        } => scan(&network, no_sweep, csv, range, subnet).await,
        Command::Read {
            device,
            address,
            csv,
            network,
        } => read(&network, device, address, csv).await,
        Command::Write {
            target,
            object,
            value,
            priority,
        } => write_cli::write_value(&target, object, &value, priority, data_dir.as_deref()).await,
        Command::Release {
            target,
            object,
            priority,
        } => write_cli::release(&target, object, priority, data_dir.as_deref()).await,
        Command::Interfaces => list_interfaces(),
    }
}

/// Started by double-click on Windows, the window closes at once and an error would be unseen.
fn wait_for_enter() {
    if cfg!(windows) {
        println!("Press Enter to close.");
        // nothing to do if the console is gone
        let _ = std::io::stdin().read_line(&mut String::new());
    }
}

fn list_interfaces() -> anyhow::Result<()> {
    let adapters = interfaces::list(false).context("cannot list network adapters")?;
    let default = interfaces::pick_default(&adapters);
    for adapter in &adapters {
        let marker = if Some(adapter) == default { "*" } else { " " };
        println!(
            "{marker} {:<15} /{:<2}  {}",
            adapter.ip.to_string(),
            adapter.prefix_len,
            adapter.name
        );
    }
    if default.is_some() {
        println!("\n* used by default; choose another with: --interface <IP>");
    }
    Ok(())
}

async fn scan(
    network: &NetworkArgs,
    no_sweep: bool,
    csv: Option<PathBuf>,
    range: Option<(u32, u32)>,
    subnet: Option<Subnet>,
) -> anyhow::Result<()> {
    let (adapter, connection) = connect(network).await?;
    let (sweep, targets) = diagnosis::plan_sweep(&adapter, subnet, !no_sweep, network.port);
    print!("Searching: Who-Is broadcast");
    if !targets.is_empty() {
        print!(", then asking {} addresses directly", targets.len());
    }
    println!(" ...\n");

    let report = scan::run_in_range(
        &connection.client,
        wait(network),
        targets,
        range,
        false,
        &|_| {},
    )
    .await?;
    print_devices(&report);
    for router in &report.routers {
        let networks: Vec<String> = router.networks.iter().map(u16::to_string).collect();
        println!(
            "Router {} reaches network(s) {}",
            present::address_text(router.address),
            networks.join(", ")
        );
    }
    let context = diagnosis::Context {
        sweep,
        link_local: adapter.ip.is_link_local(),
        port_unavailable: connection.port_unavailable.clone(),
    };
    for note in diagnosis::notes(&report, &context) {
        println!("Note: {note}");
    }
    for (address, reason) in &report.refused {
        println!("  {address}: {reason}");
    }
    if let Some(path) = csv {
        std::fs::write(&path, export::device_list_csv(&report.devices))
            .with_context(|| format!("cannot save {}", path.display()))?;
        println!("Device list saved to {}", path.display());
    }
    connection.close().await
}

async fn read(
    network: &NetworkArgs,
    device: Option<u32>,
    address: Option<Ipv4Addr>,
    csv: Option<PathBuf>,
) -> anyhow::Result<()> {
    let (connection, instance) = locate(network, device, address).await?;
    println!("Reading device {instance} ...\n");
    let reading = points::read_device(&connection.client, instance).await?;
    print_points(&reading);
    if let Some(path) = csv {
        std::fs::write(&path, export::point_list_csv(instance, &reading.points))
            .with_context(|| format!("cannot save {}", path.display()))?;
        println!("Point list saved to {}", path.display());
    }
    connection.close().await
}

async fn find_device(target: &DeviceArgs) -> anyhow::Result<(Connection, u32)> {
    locate(&target.network, Some(target.device), target.address).await
}

/// Connects and finds the device, by Who-Is for its ID or by asking its address.
async fn locate(
    network: &NetworkArgs,
    device: Option<u32>,
    address: Option<Ipv4Addr>,
) -> anyhow::Result<(Connection, u32)> {
    let (_, connection) = connect(network).await?;
    if let Some(reason) = &connection.port_unavailable {
        println!("Note: {}\n", diagnosis::port_note(reason));
    }
    let address = address.map(|ip| SocketAddrV4::new(ip, network.port));
    let instance = scan::locate(&connection.client, device, address, wait(network))
        .await
        .map_err(|error| match address {
            None => {
                anyhow!("{error:#}. If `bacnetio scan` found it \"direct\", add --address <IP>.")
            }
            Some(_) => error,
        })?;
    Ok((connection, instance))
}

async fn connect(network: &NetworkArgs) -> anyhow::Result<(Adapter, Connection)> {
    let adapter = interfaces::choose(network.interface, false)
        .map_err(|error| anyhow!("{error}; see: bacnetio interfaces"))?;
    println!(
        "Network: {} {}/{}",
        adapter.name, adapter.ip, adapter.prefix_len
    );
    let connection = connection::open(&Endpoint {
        interface: adapter.ip,
        broadcast: adapter.broadcast(),
        port: network.port,
        bbmd: network.bbmd,
    })
    .await?;
    if let Some(bbmd) = network.bbmd {
        println!("Registered as foreign device at BBMD {bbmd}");
    }
    Ok((adapter, connection))
}

fn wait(network: &NetworkArgs) -> Duration {
    Duration::from_secs(network.wait)
}

fn print_devices(report: &ScanReport) {
    if report.devices.is_empty() {
        println!("No devices found.\n");
        return;
    }
    println!("Found {} devices:", report.devices.len());
    let rows: Vec<Vec<String>> = report.devices.iter().map(device_row).collect();
    table::print(
        &["ID", "Address", "Vendor", "Model", "Name", "Found by"],
        &rows,
    );
    println!();
    for device in &report.devices {
        if let Some(error) = &device.details.error {
            println!(
                "Device {}: some details could not be read ({error})",
                device.instance
            );
        }
    }
}

fn device_row(device: &Device) -> Vec<String> {
    let ip = present::address_text(device.address);
    let address = match &device.remote {
        None => ip,
        Some(remote) => format!(
            "net {} / {} via {ip}",
            remote.network,
            present::hex(&remote.mac)
        ),
    };
    let unknown = || "?".to_string();
    vec![
        device.instance.to_string(),
        address,
        device.details.vendor.clone().unwrap_or_else(unknown),
        device.details.model.clone().unwrap_or_else(unknown),
        device.details.name.clone().unwrap_or_else(unknown),
        if device.answered_who_is {
            "Who-Is"
        } else {
            "direct"
        }
        .to_string(),
    ]
}

fn print_points(reading: &DeviceReading) {
    println!("{} objects:", reading.points.len());
    let rows: Vec<Vec<String>> = reading
        .points
        .iter()
        .map(|point| {
            vec![
                present::object_text(point.object),
                point.name.clone().unwrap_or_default(),
                present::shown_value(point),
                present::status_text(point.status),
            ]
        })
        .collect();
    table::print(&["Object", "Name", "Value", "Status"], &rows);
    println!();
    for point in &reading.points {
        if let Some(error) = &point.error {
            println!("{}: {error}", present::object_text(point.object));
        }
    }
    for note in &reading.notes {
        println!("Note: {note}");
    }
}
