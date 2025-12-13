# Layer 1 Runtime SDK Licensing

This document clarifies the licensing terms for the **Layer 1 (Low-Level Runtime SDK)** component of Control Path.

## Overview

Control Path uses a two-layer SDK architecture:

- **Layer 1**: Low-Level Runtime SDK (`@controlpath/runtime`) - Core runtime library for AST loading and flag evaluation
- **Layer 2**: Generated Type-Safe SDKs - Generated from your flag definitions, specific to your project

## Layer 1 SDK Licensing

The Layer 1 Runtime SDK (`@controlpath/runtime`) is licensed under the **Elastic License 2.0** (see [LICENSE](../LICENSE) for the full license text).

### Usage Rights

You may:

- ✅ Use the Layer 1 SDK as a dependency in your projects
- ✅ Use the Layer 1 SDK to build and run your applications
- ✅ Modify the Layer 1 SDK for your own use (subject to license terms)

### Redistribution Restrictions

**You may NOT redistribute the Layer 1 SDK separately** (e.g., as a standalone package, republished to npm, etc.). The Layer 1 SDK is owned and distributed by Release Workshop Ltd.

However, you may:

- ✅ Include the Layer 1 SDK as a dependency in your application
- ✅ Distribute your application that includes the Layer 1 SDK as a dependency
- ✅ Use the Layer 1 SDK in your commercial products

### What This Means

- **As a dependency**: You can add `@controlpath/runtime` to your `package.json` and use it in your projects
- **In your application**: Your application can depend on the Layer 1 SDK and be distributed with it included
- **Not as a standalone package**: You cannot republish, fork, or redistribute the Layer 1 SDK as a separate package

## Layer 2 Generated SDKs

Layer 2 SDKs are generated from your flag definitions using the Control Path CLI. These generated SDKs:

- ✅ Can be included in your application code
- ✅ Can be redistributed with your application
- ✅ Are specific to your project and flag definitions
- ✅ Are generated code, not the core runtime

## Questions?

If you have questions about licensing or redistribution rights, please contact Release Workshop Ltd.

---

**Copyright © 2024-2025 Release Workshop Ltd. All rights reserved.**
