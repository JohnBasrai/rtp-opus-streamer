# Test Audio Samples

Sample files from [pdx-cs-sound/wavs](https://github.com/pdx-cs-sound/wavs) by Bart Massey.
Licensed under Creative Commons CC-0.

## Files

* `voice.wav` - Short speech sample for testing voice-optimized codec configuration
* `music.wav` - Acoustic guitar sample for testing music content

## Current Behavior

The system is currently optimized for voice transmission (16kHz, 24 kbps, VOIP mode):
- ✅ `voice.wav` sounds clear and natural
- ⚠️  `music.wav` sounds degraded (scratchy/muffled) - **this is expected**

Music quality will be addressed in a future update with configurable codec modes.

## Technical Specs

Both files are provided at 48kHz stereo to test the codec's resampling and channel conversion.
Current voice-optimized settings downsample to 16kHz mono.
