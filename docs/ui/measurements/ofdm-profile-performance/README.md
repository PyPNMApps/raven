# OFDM Profile Performance

[User guide](../../README.md) · [Technical design](../../../development/ofdm-profile-performance.md)

Use **OFDM Profile Performance** in the left navigation to test a captured
modulation profile against measured RxMER, with optional channel estimation.
This is a decoder simulation rather than counters collected from your modem.

1. Click **Select captures…** (the input popup opens on first use).
2. Use **Browse…** to select a modulation-profile PNM file and an RxMER PNM file.
   The buttons open the operating system's file picker in the demo folder.
3. Optionally select a downstream channel-estimation PNM file. Enable **Use MCE**
   to estimate receiver correction from received pilots. Leave it off to use the
   supplied correction. Without a channel file, the simulation uses a unity channel.
4. Check the captured profiles to test (all are initially checked), and set the
   maximum capture separation in seconds.
   The default is one day; enter zero to require matching profile/RxMER timestamps.
5. Click **Continue**. The MAC address, channel and frequency grids must match.
   A bad input opens an error popup and leaves the input popup available to fix it.

The bundled **files → demo → ofdm-profile-performance** folder contains
`modulation_profile.bin`, `rxmer.bin`, `channel_estimation.bin` and a walkthrough.
Use these together with the default time tolerance. See the
[installed demo locations](../../examples.md#ofdm-profile-performance).
Close dismisses input selection without running a simulation. File selection does
not modify captures. This popup currently accepts binary PNM captures.

The main graph overlays measured RxMER and the checked profiles' **Shannon
references (dB)**. That reference shows the ideal relationship to modulation size;
it is not a guaranteed FEC threshold. PLC carriers use pale-yellow shading in dark mode and a warmer amber-yellow
shade in light mode to stand out against the white graph. Drag a section of
the plot to zoom and double-click to restore the full frequency range.

Use the workspace's **Profiles to test** checkboxes to choose any combination of
prepared profiles. Each checkbox controls that profile's overlay and inclusion in
the next test. Clearing every box disables FEC Performance; selecting a box enables
it again. Changing the selection clears previous results. To include a profile
that was not prepared, reopen Select captures and check it before Continue.

Choose **FEC Performance** to run all checked profiles sequentially. The popup
keeps separate FEC and LDPC table rows for each profile; it does not pool their
counters or BER. It displays:

- FEC codeword counts, compared bits, bit errors and BER for the observation window.
- LDPC mean and total iterations, failures, and NCP/PLC iterations for all completed
  codewords, including completion/drain. BCH failures use this same completed scope.
- The actual run status, including cancellation or runtime limits. A dash means
  unavailable; it does not mean zero errors.

The current observation is 10 ms per profile, with a 300-second host limit per
profile. A multi-profile run may take longer because it simulates
transmission, acquisition and decoding. **Stop** requests cancellation. Closing
the results popup leaves the run active; reopen it with FEC Performance after
completion or select Run again in the popup for a new run of all checked profiles. Change inputs through
Select captures and Continue before testing a different profile or receiver mode.

## Workspace

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="images/ofdm-profile-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="images/ofdm-profile-light.png">
  <img alt="OFDM Profile Performance with measured RxMER, profile reference, PLC shading and MCE selected" src="images/ofdm-profile-dark.png" width="1000">
</picture>

## FEC and LDPC results

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="images/ofdm-profile-fec-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="images/ofdm-profile-fec-light.png">
  <img alt="Actual MERLIN FEC and LDPC counters from the bundled captures" src="images/ofdm-profile-fec-dark.png" width="760">
</picture>

Screenshots show the actual Ubuntu Qt workspace and results content, using the
bundled single captures and Profiles 0–3 with MCE. They are not modem measurements
of FEC counters or independent Windows/macOS runtime evidence.
