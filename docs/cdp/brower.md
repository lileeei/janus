# Chrome DevTools Protocol - Browser Domain

## Methods

### Browser.addPrivacySandboxCoordinatorKeyConfig
Configures encryption keys used with a given privacy sandbox API to talk to a trusted coordinator.

**Parameters:**
- `api` (PrivacySandboxAPI)
- `coordinatorOrigin` (string)
- `keyConfig` (string)
- `browserContextId` (BrowserContextID) - *Optional*

### Browser.addPrivacySandboxEnrollmentOverride
Allows a site to use privacy sandbox features that require enrollment without the site actually being enrolled.

**Parameters:**
- `url` (string)

### Browser.close
Close browser gracefully.

### Browser.getVersion
Returns version information.

**Returns:**
- `protocolVersion` (string)
- `product` (string)
- `revision` (string)
- `userAgent` (string)
- `jsVersion` (string)

### Browser.resetPermissions
Reset all permission management for all origins.

**Parameters:**
- `browserContextId` (BrowserContextID) - *Optional*

### Browser.cancelDownload (Experimental)
Cancel a download if in progress.

**Parameters:**
- `guid` (string)
- `browserContextId` (BrowserContextID) - *Optional*

### Browser.crash (Experimental)
Crashes browser on the main thread.

### Browser.crashGpuProcess (Experimental)
Crashes GPU process.

### Browser.executeBrowserCommand (Experimental)
Invoke custom browser commands used by telemetry.

**Parameters:**
- `commandId` (BrowserCommandId)

### Browser.getBrowserCommandLine (Experimental)
Returns the command line switches for the browser process.

**Returns:**
- `arguments` (array[string])

### Browser.getHistogram (Experimental)
Get a Chrome histogram by name.

**Parameters:**
- `name` (string)
- `delta` (boolean) - *Optional*

**Returns:**
- `histogram` (Histogram)

### Browser.getHistograms (Experimental)
Get Chrome histograms.

**Parameters:**
- `query` (string) - *Optional*
- `delta` (boolean) - *Optional*

**Returns:**
- `histograms` (array[Histogram])

### Browser.getWindowBounds (Experimental)
Get position and size of the browser window.

**Parameters:**
- `windowId` (WindowID)

**Returns:**
- `bounds` (Bounds)

### Browser.getWindowForTarget (Experimental)
Get the browser window that contains the devtools target.

**Parameters:**
- `targetId` (Target.TargetID)

**Returns:**
- `windowId` (WindowID)
- `bounds` (Bounds)

### Browser.grantPermissions (Experimental)
Grant specific permissions to the given origin and reject all others.

**Parameters:**
- `permissions` (array[PermissionType])
- `origin` (string) - *Optional*
- `browserContextId` (BrowserContextID) - *Optional*

### Browser.setDockTile (Experimental)
Set dock tile details, platform-specific.

**Parameters:**
- `badgeLabel` (string) - *Optional*
- `image` (string) - *Optional*

### Browser.setDownloadBehavior (Experimental)
Set the behavior when downloading a file.

**Parameters:**
- `behavior` (string) - Allowed: `deny`, `allow`, `allowAndName`, `default`
- `browserContextId` (BrowserContextID) - *Optional*
- `downloadPath` (string) - *Optional*
- `eventsEnabled` (boolean) - *Optional*

### Browser.setPermission (Experimental)
Set permission settings for given origin.

**Parameters:**
- `permission` (PermissionDescriptor)
- `setting` (PermissionSetting)
- `origin` (string) - *Optional*
- `browserContextId` (BrowserContextID) - *Optional*

### Browser.setWindowBounds (Experimental)
Set position and/or size of the browser window.

**Parameters:**
- `windowId` (WindowID)
- `bounds` (Bounds)

## Events

### Browser.downloadProgress (Experimental)
Fired when download makes progress.

**Parameters:**
- `guid` (string)
- `totalBytes` (number)
- `receivedBytes` (number)
- `state` (string) - Allowed: `inProgress`, `completed`, `canceled`

### Browser.downloadWillBegin (Experimental)
Fired when page is about to start a download.

**Parameters:**
- `frameId` (Page.FrameId)
- `guid` (string)
- `url` (string)
- `suggestedFilename` (string)

## Types

### Browser.Bounds (Experimental)
Browser window bounds information.

**Properties:**
- `left` (integer) - *Optional*
- `top` (integer) - *Optional*
- `width` (integer) - *Optional*
- `height` (integer) - *Optional*
- `windowState` (WindowState) - *Optional*

### Browser.BrowserCommandId (Experimental)
Browser command ids.

**Allowed Values:**
- `openTabSearch`
- `closeTabSearch`

### Browser.BrowserContextID
Type: string

### Browser.Bucket (Experimental)
Chrome histogram bucket.

**Properties:**
- `low` (integer)
- `high` (integer)
- `count` (integer)

### Browser.Histogram (Experimental)
Chrome histogram.

**Properties:**
- `name` (string)
- `sum` (integer)
- `count` (integer)
- `buckets` (array[Bucket])

### Browser.PermissionDescriptor
Definition of PermissionDescriptor.

**Properties:**
- `name` (string)
- `sysex` (boolean) - *Optional*
- `userVisibleOnly` (boolean) - *Optional*
- `allowWithoutSanitization` (boolean) - *Optional*
- `allowWithoutGesture` (boolean) - *Optional*
- `panTiltZoom` (boolean) - *Optional*

### Browser.PermissionSetting
**Allowed Values:**
- `granted`
- `denied`
- `prompt`

### Browser.PermissionType
**Allowed Values:**
- `ar`, `audioCapture`, `automaticFullscreen`, `backgroundFetch`, `backgroundSync`, `cameraPanTiltZoom`, `capturedSurfaceControl`, `clipboardReadWrite`, `clipboardSanitizedWrite`, `displayCapture`, `durableStorage`, `geolocation`, `handTracking`, `idleDetection`, `keyboardLock`, `localFonts`, `localNetworkAccess`, `midi`, `midiSysex`, `nfc`, `notifications`, `paymentHandler`, `periodicBackgroundSync`, `pointerLock`, `protectedMediaIdentifier`, `sensors`, `smartCard`, `speakerSelection`, `storageAccess`, `topLevelStorageAccess`, `videoCapture`, `vr`, `wakeLockScreen`, `wakeLockSystem`, `webAppInstallation`, `webPrinting`, `windowManagement`

### Browser.PrivacySandboxAPI (Experimental)
**Allowed Values:**
- `BiddingAndAuctionServices`
- `TrustedKeyValue`

### Browser.WindowID
Type: integer

### Browser.WindowState
The state of the browser window.

**Allowed Values:**
- `normal`
- `minimized`
- `maximized`
- `fullscreen`
