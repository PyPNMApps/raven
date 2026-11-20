# Multi-capture channel estimation demo

75 real lab captures across two channels of one sanitized device.

1. Choose DS Channel Estimation → Multi Capture in the left gutter.
2. Choose this folder, or open channel-estimation/multiple-captures.zip.
3. Select MAC 00:ca:00:00:00:01 and a channel in the tree.
4. Compare Min, Avg I/Q and Max magnitude; switch to Group delay.
5. Drag across either graph to zoom; double-click or Reset zoom restores it.

Group delay comes from the complex averaged coefficients, not averaged delays.
Only the CM MAC field changed. Coefficients and original capture times are retained.
See provenance.json for input hashes and the exact changes. These examples are
user-supplied lab data, not synthetic impairment scenarios. Export into your own
writable folder; packaged demos are read-only.
