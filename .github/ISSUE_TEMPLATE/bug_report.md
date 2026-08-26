---
name: Bug report
about: Something works incorrectly or crashes
title: "[Bug] "
labels: bug
assignees: ''
---

**Describe the Bug**

A clear and concise description of what is wrong.

**To Reproduce**

Steps to reproduce the behavior:

1. Start the app with driver parameters: …
2. Select enclosure type: …
3. Change parameter: …
4. Observe: …

**Expected Behavior**

What you expected to happen instead.

**Numbers In / Numbers Out**

Please paste the driver T/S parameters and enclosure parameters you used,
and what the application showed vs. what you expected:

```text
Driver: Fs=…, Qms=…, Qes=…, Vas=…, Sd=…, Re=…, Le=…, Xmax=…
Enclosure: type=…, parameters=…
Application shows: …
Expected: …
```

**Environment**

- OS: [e.g., Windows 11, Ubuntu 24.04, macOS 15]
- App version or commit: [e.g., 0.1.0]
- How you run it: [built from source / release binary]

**Additional Context**

Attach the `.spkproj` project file if possible (it is plain JSON).
Screenshots of the graphs help a lot.
