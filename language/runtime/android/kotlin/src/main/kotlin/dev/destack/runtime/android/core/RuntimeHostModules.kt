package dev.destack.runtime.android.core

import dev.destack.runtime.android.module.background.BackgroundEvents
import dev.destack.runtime.android.module.background.BackgroundRequests
import dev.destack.runtime.android.module.lifecycle.LifecycleEvents
import dev.destack.runtime.android.module.text.TextEvents
import dev.destack.runtime.android.module.text.TextRequests

/**
 * The lifecycle host surface attached to one Android runtime host.
 */
public class LifecycleHost(
    events: LifecycleEvents,
) : LifecycleEvents by events
