# Rekordbox Monitor

A Rust application that monitors real-time data from Rekordbox DJ software, including track information, fader positions, and beat timing.

## Features

- Real-time monitoring of:
  - Currently playing tracks (both decks)
  - Track metadata (title, artist, ID)
  - Beat positions and cue points
  - Channel faders and crossfader positions
- Integration with Rekordbox collection XML for enhanced track information
- Visual representation of mixer state

## How It Works

The application uses memory reading techniques to access Rekordbox's runtime data:

1. **Process Attachment**: Connects to the running Rekordbox process (`rekordbox.exe`) and locates the necessary memory regions.

2. **Data Reading**: Uses memory pointer chains to read various pieces of information:
   - Track metadata (titles, artists, IDs)
   - Beat positions and timing information
   - Mixer state (fader positions)

3. **XML Integration**: Parses Rekordbox's collection XML file to provide additional track information and cue point data, particularly for:
   - Track identification
   - Cue points with "EW" markers
   - Beat timing verification

## Usage

1. Ensure Rekordbox is running
2. Launch the application with the path to your Rekordbox collection XML:

```
cargo run -- --collection-xml-path /path/to/collection.xml
```
