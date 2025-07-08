# SIP Client

A modern GUI SIP client built with [Rust](https://www.rust-lang.org/), [Dioxus](https://github.com/DioxusLabs/dioxus) for the UI, and [rvoip](https://github.com/eisenzopf/rvoip) for SIP functionality.

## Features

- 🖥️ **Modern Desktop GUI** - Built with Dioxus for cross-platform desktop support
- 📞 **Full SIP Support** - Powered by the rvoip library, a comprehensive Rust VoIP stack
- 🔐 **Security First** - Pure Rust implementation with modern security practices
- 🎵 **Audio Codecs** - Support for OPUS, PCMU, and PCMA codecs
- 📡 **SIP Registration** - Standard SIP registration with authentication
- 📞 **Call Management** - Make and receive calls with full call state management
- 🎛️ **Real-time Status** - Live status updates and call information

## Architecture

This application uses a layered architecture:

- **UI Layer**: Dioxus-based desktop application
- **SIP Client Layer**: Wrapper around rvoip functionality
- **rvoip Stack**: Modern Rust VoIP implementation with:
  - SIP protocol handling (RFC 3261 compliant)
  - RTP/RTCP media transport
  - Audio codec support
  - Event-driven architecture

## Prerequisites

- Rust 1.70+ 
- macOS, Linux, or Windows
- Audio system support (for call audio)

## Building

1. Clone the repository:
```bash
git clone <your-repo-url>
cd sip_client
```

2. Build the project:
```bash
cargo build --release
```

3. Run the application:
```bash
cargo run
```

## Usage

### Configuration

1. **Username**: Your SIP username/extension
2. **Password**: Your SIP password
3. **SIP Server URI**: Your SIP server address (e.g., `sip:pbx.example.com:5060`)
4. **Local Port**: Local port for SIP communication (default: 5070)

### Registration

1. Fill in your SIP credentials in the Configuration section
2. Click the "Register" button
3. Wait for registration confirmation (status will show "Registered ✅")

### Making Calls

1. Ensure you are registered with the SIP server
2. Enter the target SIP URI in the "Call Target" field (e.g., `sip:1001@example.com`)
3. Click the "📞 Call" button
4. The call status will be displayed in real-time

### Receiving Calls

- Incoming calls will automatically appear in the Call Control section
- Click "✅ Answer" to accept the call
- Click "❌ Decline" to reject the call

## Configuration Examples

### Asterisk/FreePBX
```
Username: 1000
Password: your_extension_password
SIP Server URI: sip:192.168.1.100:5060
Local Port: 5070
```

### 3CX
```
Username: extension_number
Password: extension_password
SIP Server URI: sip:3cx.example.com:5060
Local Port: 5070
```

## Development

### Project Structure

```
src/
├── main.rs          # Application entry point
├── sip_client.rs    # SIP client wrapper around rvoip
└── ui.rs            # Dioxus UI components
```

### Key Dependencies

- **dioxus**: Desktop GUI framework
- **rvoip**: Modern Rust VoIP stack
- **tokio**: Async runtime
- **log/env_logger**: Logging
- **anyhow**: Error handling

### Running in Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with info logging
RUST_LOG=info cargo run
```

## Troubleshooting

### Common Issues

1. **Registration fails**: 
   - Check SIP server URI format
   - Verify credentials
   - Check network connectivity
   - Ensure local port is not blocked

2. **No audio during calls**:
   - Check system audio settings
   - Verify codec compatibility
   - Check firewall settings for RTP ports

3. **Can't make calls**:
   - Ensure you're registered first
   - Check target URI format
   - Verify server allows outbound calls

### Logging

Enable debug logging to see detailed SIP messages:

```bash
RUST_LOG=debug cargo run
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- [rvoip](https://github.com/eisenzopf/rvoip) - Modern Rust VoIP stack
- [Dioxus](https://github.com/DioxusLabs/dioxus) - Rust GUI framework
- [Tokio](https://tokio.rs/) - Async runtime for Rust 