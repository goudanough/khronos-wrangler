# Khronos-Wrangler
For the game we're currently developing we need access to the Scene API and Depth API provided in the Meta Quest 3 OpenXR SDK.
Annoyingly, these APIs don't seem to have been ratified into the Khronos Registry. Or even to be in the process of being ratified (accoring to Tom Deakin at least).

OpenXR definitions are stored in XML, which can be processed into C headers - or in our case - rust bindings, through the openxrs crates.
Fortunately the Meta Quest OpenXR SDK *does* come with C headers, so with a little magic we can reverse these C headers into their original XML (give or take a few details).
