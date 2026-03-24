package dev.destack.runtime.android

import android.os.Bundle

import androidx.activity.result.ActivityResultRegistry
import androidx.activity.result.contract.ActivityResultContract
import androidx.core.app.ActivityOptionsCompat

import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import androidx.savedstate.SavedStateRegistry
import androidx.savedstate.SavedStateRegistryController
import androidx.savedstate.SavedStateRegistryOwner

/**
 * One test lifecycle owner for Android embedder tests.
 */
internal class TestLifecycleOwner : LifecycleOwner {
    private val lifecycleRegistry = LifecycleRegistry.createUnsafe(this)

    override val lifecycle: Lifecycle
        get() = lifecycleRegistry

    /**
     * Send one lifecycle event through this owner.
     */
    fun handleEvent(
        event: Lifecycle.Event,
    ) {
        lifecycleRegistry.handleLifecycleEvent(event)
    }
}

/**
 * One test saved-state owner for Android embedder tests.
 */
internal class TestSavedStateOwner(
    restoredState: Bundle? = null,
) : LifecycleOwner, SavedStateRegistryOwner {
    private val lifecycleRegistry = LifecycleRegistry.createUnsafe(this)
    private val savedStateController = SavedStateRegistryController.create(this)

    init {
        savedStateController.performAttach()
        savedStateController.performRestore(restoredState)
    }

    override val lifecycle: Lifecycle
        get() = lifecycleRegistry

    override val savedStateRegistry: SavedStateRegistry
        get() = savedStateController.savedStateRegistry

    /**
     * Send one lifecycle event through this owner.
     */
    fun handleEvent(
        event: Lifecycle.Event,
    ) {
        lifecycleRegistry.handleLifecycleEvent(event)
    }

    /**
     * Save one snapshot of the current owner state.
     */
    fun saveState(): Bundle {
        val bundle = Bundle()

        savedStateController.performSave(bundle)

        return bundle
    }
}

/**
 * One recording Android activity-result registry for embedder tests.
 */
internal class RecordingActivityResultRegistry : ActivityResultRegistry() {
    val launchedContracts: MutableList<String> = mutableListOf()
    val launchedInputs: MutableMap<Int, Any?> = mutableMapOf()
    val launchedRequestCodes: MutableList<Int> = mutableListOf()

    override fun <I, O> onLaunch(
        requestCode: Int,
        contract: ActivityResultContract<I, O>,
        input: I,
        options: ActivityOptionsCompat?,
    ) {
        launchedRequestCodes += requestCode
        launchedContracts += contract.javaClass.name
        launchedInputs[requestCode] = input
    }

    /**
     * Complete the most recent launcher request.
     */
    fun <O> completeLastLaunch(
        result: O,
    ) {
        dispatchResult(launchedRequestCodes.last(), result)
    }
}
