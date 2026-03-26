import Foundation
import RuntimeHostAppleCore

#if canImport(UIKit)
  import UIKit

  /// The iOS text-input request surface backed by one hidden UIKit editor.
  @MainActor
  public final class UIKitTextInputRequests: NSObject, TextInputRequests, UITextViewDelegate {
    /// One hidden UIKit editor that answers geometry queries from renderer hints.
    private final class HostTextInputTextView: UITextView {
      /// The latest renderer geometry.
      var runtimeGeometry: RuntimeHostTextInputGeometry?
      /// The latest renderer state.
      var runtimeState: RuntimeHostTextInputState?

      override func caretRect(
        for position: UITextPosition
      ) -> CGRect {
        guard
          let geometry = runtimeGeometry,
          let caretRectangle = geometry.caretRectangle
        else {
          return super.caretRect(for: position)
        }

        return projectedLocalRectangle(
          rectangle: caretRectangle,
          geometry: geometry
        ) ?? super.caretRect(for: position)
      }

      override func firstRect(
        for range: UITextRange
      ) -> CGRect {
        guard
          let geometry = runtimeGeometry,
          let state = runtimeState
        else {
          return super.firstRect(for: range)
        }

        // composing
        if let composing = state.composing,
          let composingTextRange = textRange(for: composing),
          composingTextRange.start == range.start,
          composingTextRange.end == range.end,
          let composingRectangle = geometry.composingRectangle,
          let localRectangle = projectedLocalRectangle(
            rectangle: composingRectangle,
            geometry: geometry
          )
        {
          return localRectangle
        }

        // collapsed selection
        if let selectionTextRange = selectedTextRange,
          selectionTextRange.start == range.start,
          selectionTextRange.end == range.end,
          selectionTextRange.isEmpty,
          let caretRectangle = geometry.caretRectangle,
          let localRectangle = projectedLocalRectangle(
            rectangle: caretRectangle,
            geometry: geometry
          )
        {
          return localRectangle
        }

        return super.firstRect(for: range)
      }

      /// Build one UIKit text range from one runtime text range.
      private func textRange(
        for range: RuntimeHostTextInputRange
      ) -> UITextRange? {
        guard
          let start = position(
            from: beginningOfDocument,
            offset: range.startOffset
          )
        else {
          return nil
        }
        guard
          let end = position(
            from: beginningOfDocument,
            offset: range.endOffset
          )
        else {
          return nil
        }

        return textRange(
          from: start,
          to: end
        )
      }

      /// Project one runtime rectangle into local UIKit editor coordinates.
      private func projectedLocalRectangle(
        rectangle: RuntimeHostTextInputRectangle,
        geometry: RuntimeHostTextInputGeometry
      ) -> CGRect? {
        let editorFrame = projectedTargetRectangle(
          rectangle: geometry.editorRectangle,
          transform: geometry.localToTargetTransform
        )
        let localFrame = projectedTargetRectangle(
          rectangle: rectangle,
          transform: geometry.localToTargetTransform
        )

        return CGRect(
          x: localFrame.minX - editorFrame.minX,
          y: localFrame.minY - editorFrame.minY,
          width: max(localFrame.width, 1),
          height: max(localFrame.height, 1)
        )
      }

      /// Project one runtime rectangle into target-local coordinates.
      private func projectedTargetRectangle(
        rectangle: RuntimeHostTextInputRectangle,
        transform: RuntimeHostTextInputTransform2D
      ) -> CGRect {
        let transform = CGAffineTransform(
          a: transform.xx,
          b: transform.yx,
          c: transform.xy,
          d: transform.yy,
          tx: transform.tx,
          ty: transform.ty
        )
        let rectangle = CGRect(
          x: rectangle.x,
          y: rectangle.y,
          width: rectangle.width,
          height: rectangle.height
        )

        return rectangle.applying(transform).integral
      }
    }

    /// One tracked text-session record.
    private struct SessionRecord {
      /// The session configuration.
      let configuration: RuntimeHostTextInputConfiguration
      /// The latest geometry hint.
      var geometry: RuntimeHostTextInputGeometry?
      /// The latest renderer-owned text state.
      var state: RuntimeHostTextInputState
    }

    private let events: any TextInputEvents
    private var sessions: [UInt64: SessionRecord] = [:]
    private var activeSessionID: UInt64?
    private var isApplyingHostState: Bool = false
    private weak var hostView: UIView?
    private var hiddenTextView: HostTextInputTextView?

    /// Create one UIKit text-input request surface.
    public init(
      events: any TextInputEvents
    ) {
      self.events = events
      super.init()
    }

    public func openTextInput(
      _ request: RuntimeHostTextInputOpenRequest
    ) -> UInt32 {
      guard let textView = ensureHiddenTextView() else {
        return hostStatusNotSupported
      }

      let sessionID = request.configuration.sessionID
      let geometry = sessions[sessionID]?.geometry

      // open or replace the tracked session
      sessions[sessionID] = SessionRecord(
        configuration: request.configuration,
        geometry: geometry,
        state: request.state
      )
      activeSessionID = sessionID

      // apply the full host editor shape before focusing
      configureTextView(
        textView,
        configuration: request.configuration
      )
      if let geometry {
        applyTextInputGeometry(
          textView,
          geometry: geometry
        )
      }
      applyTextInputState(
        textView,
        state: request.state
      )
      textView.becomeFirstResponder()

      return hostStatusOk
    }

    public func closeTextInput(
      _ request: RuntimeHostTextInputCloseRequest
    ) -> UInt32 {
      guard sessions.removeValue(forKey: request.sessionID) != nil else {
        return hostStatusNotFound
      }

      // clear the focused session when the closed session was active
      if activeSessionID == request.sessionID {
        activeSessionID = nil
        hiddenTextView?.resignFirstResponder()
      }

      return hostStatusOk
    }

    public func setTextInputGeometry(
      _ request: RuntimeHostTextInputGeometryRequest
    ) -> UInt32 {
      guard var session = sessions[request.sessionID] else {
        return hostStatusNotFound
      }

      // record the next renderer geometry for this session
      session.geometry = request.geometry
      sessions[request.sessionID] = session

      // keep the active editor aligned with the renderer hint
      if activeSessionID == request.sessionID, let textView = hiddenTextView {
        applyTextInputGeometry(
          textView,
          geometry: request.geometry
        )
      }

      return hostStatusOk
    }

    public func setTextInputState(
      _ request: RuntimeHostTextInputStateRequest
    ) -> UInt32 {
      guard var session = sessions[request.sessionID] else {
        return hostStatusNotFound
      }

      // record the next renderer state for this session
      session.state = request.state
      sessions[request.sessionID] = session

      // push renderer-owned state into the active host editor
      if activeSessionID == request.sessionID, let textView = hiddenTextView {
        applyTextInputState(
          textView,
          state: request.state
        )
      }

      return hostStatusOk
    }

    public func textViewDidChange(
      _ textView: UITextView
    ) {
      handleEditorStateChanged(textView)
    }

    public func textViewDidChangeSelection(
      _ textView: UITextView
    ) {
      handleEditorStateChanged(textView)
    }

    public func textView(
      _ textView: UITextView,
      shouldChangeTextIn range: NSRange,
      replacementText text: String
    ) -> Bool {
      let _ = range

      guard let sessionID = activeSessionID, let session = sessions[sessionID] else {
        return true
      }

      // keep single-line sessions from inserting line breaks
      if !session.configuration.isMultiline && text == "\n" {
        textView.resignFirstResponder()

        return false
      }

      return true
    }

    /// Resolve or install the hidden UIKit editor.
    private func ensureHiddenTextView() -> HostTextInputTextView? {
      guard let hostView = resolveHostView() else {
        return nil
      }

      // reuse the existing editor when it is already attached
      if let textView = hiddenTextView {
        if textView.superview !== hostView {
          textView.removeFromSuperview()
          hostView.addSubview(textView)
        }

        self.hostView = hostView

        return textView
      }

      // install one hidden editor into the active surface view
      let textView = HostTextInputTextView(frame: .zero)
      textView.delegate = self
      textView.alpha = 0.01
      textView.backgroundColor = .clear
      textView.isScrollEnabled = false
      textView.autocapitalizationType = .sentences
      textView.autocorrectionType = .default
      textView.spellCheckingType = .default
      textView.smartDashesType = .no
      textView.smartQuotesType = .no
      textView.smartInsertDeleteType = .no
      textView.textContainerInset = .zero
      textView.textContainer.lineFragmentPadding = 0
      textView.accessibilityElementsHidden = true
      textView.frame = CGRect(x: 0, y: 0, width: 1, height: 1)
      hostView.addSubview(textView)
      self.hostView = hostView
      hiddenTextView = textView

      return textView
    }

    /// Resolve the active UIKit host view for text-input attachment.
    private func resolveHostView() -> UIView? {
      let connectedScenes = UIApplication.shared.connectedScenes
        .compactMap { $0 as? UIWindowScene }

      // prefer the active key window when one is available
      for scene in connectedScenes {
        if let window = scene.windows.first(where: \.isKeyWindow) {
          return window.rootViewController?.view ?? window
        }
      }

      // otherwise fall back to the first visible window
      for scene in connectedScenes {
        if let window = scene.windows.first(where: { !$0.isHidden }) {
          return window.rootViewController?.view ?? window
        }
      }

      return nil
    }

    /// Configure the hidden editor for one text-session configuration.
    private func configureTextView(
      _ textView: UITextView,
      configuration: RuntimeHostTextInputConfiguration
    ) {
      textView.keyboardType = keyboardType(for: configuration.inputType)
      textView.isSecureTextEntry = configuration.isSecure
      textView.returnKeyType = configuration.isMultiline ? .default : .done
      textView.autocorrectionType = configuration.inputType == .password ? .no : .default
      textView.textContentType = configuration.isSecure ? .password : nil
    }

    /// Apply one renderer geometry hint to the hidden editor.
    private func applyTextInputGeometry(
      _ textView: HostTextInputTextView,
      geometry: RuntimeHostTextInputGeometry
    ) {
      textView.runtimeGeometry = geometry

      let editorFrame = CGRect(
        x: geometry.editorRectangle.x,
        y: geometry.editorRectangle.y,
        width: max(geometry.editorRectangle.width, 1),
        height: max(geometry.editorRectangle.height, 1)
      )
      let transform = CGAffineTransform(
        a: geometry.localToTargetTransform.xx,
        b: geometry.localToTargetTransform.yx,
        c: geometry.localToTargetTransform.xy,
        d: geometry.localToTargetTransform.yy,
        tx: geometry.localToTargetTransform.tx,
        ty: geometry.localToTargetTransform.ty
      )

      textView.frame = editorFrame.applying(transform).integral
    }

    /// Apply one renderer-owned text state to the hidden editor.
    private func applyTextInputState(
      _ textView: HostTextInputTextView,
      state: RuntimeHostTextInputState
    ) {
      isApplyingHostState = true
      defer { isApplyingHostState = false }

      // geometry queries
      textView.runtimeState = state

      // apply the full editor contents first
      textView.text = state.text

      // apply one active composing span when present
      if let composing = clampedRange(
        state.composing,
        text: state.text
      ) {
        let composingText = substring(
          state.text,
          range: composing
        )
        let selectedRange = selectedRangeWithinComposing(
          selection: clampedRange(
            state.selection,
            text: state.text
          ),
          composing: composing
        )

        if let composingTextRange = textRange(
          in: textView,
          range: composing
        ) {
          textView.selectedTextRange = composingTextRange
        }

        textView.setMarkedText(
          composingText,
          selectedRange: selectedRange
        )
      } else if textView.markedTextRange != nil {
        textView.unmarkText()
      }

      // apply the final selection after the text state is in place
      if let selection = clampedRange(
        state.selection,
        text: state.text
      ),
        let selectionTextRange = textRange(
          in: textView,
          range: selection
        )
      {
        textView.selectedTextRange = selectionTextRange
      }
    }

    /// Handle one host editor state change for the active session.
    private func handleEditorStateChanged(
      _ textView: UITextView
    ) {
      guard !isApplyingHostState else {
        return
      }
      guard let sessionID = activeSessionID, var session = sessions[sessionID] else {
        return
      }

      // materialize the latest host-owned text state
      let state = readTextInputState(textView)
      session.state = state
      sessions[sessionID] = session

      events.sendTextInputEvent(
        RuntimeHostTextInputEvent(
          sessionID: sessionID,
          state: state
        )
      )
    }

    /// Read one text-session state snapshot from the hidden editor.
    private func readTextInputState(
      _ textView: UITextView
    ) -> RuntimeHostTextInputState {
      let text = textView.text ?? ""
      let selection = selectionRange(in: textView)
      let composing = composingRange(in: textView)

      return RuntimeHostTextInputState(
        text: text,
        selection: selection,
        composing: composing
      )
    }

    /// Read one selection range from the hidden editor.
    private func selectionRange(
      in textView: UITextView
    ) -> RuntimeHostTextInputRange {
      let selectedRange = textView.selectedRange
      let startOffset = max(selectedRange.location, 0)
      let endOffset = max(selectedRange.location + selectedRange.length, startOffset)

      return RuntimeHostTextInputRange(
        startOffset: startOffset,
        endOffset: endOffset
      )
    }

    /// Read one composing range from the hidden editor.
    private func composingRange(
      in textView: UITextView
    ) -> RuntimeHostTextInputRange? {
      guard let markedTextRange = textView.markedTextRange else {
        return nil
      }

      let startOffset = textView.offset(
        from: textView.beginningOfDocument,
        to: markedTextRange.start
      )
      let endOffset = textView.offset(
        from: textView.beginningOfDocument,
        to: markedTextRange.end
      )

      return RuntimeHostTextInputRange(
        startOffset: max(startOffset, 0),
        endOffset: max(endOffset, startOffset)
      )
    }

    /// Build one UIKit text range from one runtime range.
    private func textRange(
      in textView: UITextView,
      range: RuntimeHostTextInputRange
    ) -> UITextRange? {
      guard
        let start = textView.position(
          from: textView.beginningOfDocument,
          offset: range.startOffset
        )
      else {
        return nil
      }
      guard
        let end = textView.position(
          from: textView.beginningOfDocument,
          offset: range.endOffset
        )
      else {
        return nil
      }

      return textView.textRange(
        from: start,
        to: end
      )
    }

    /// Map one runtime input type to one UIKit keyboard type.
    private func keyboardType(
      for inputType: RuntimeHostTextInputType
    ) -> UIKeyboardType {
      switch inputType {
      case .text:
        return .default
      case .number:
        return .numberPad
      case .email:
        return .emailAddress
      case .url:
        return .URL
      case .password:
        return .default
      case .phone:
        return .phonePad
      case .search:
        return .webSearch
      }
    }

    /// Clamp one runtime text range against one Swift string length.
    private func clampedRange(
      _ range: RuntimeHostTextInputRange?,
      text: String
    ) -> RuntimeHostTextInputRange? {
      guard let range else {
        return nil
      }

      let textLength = (text as NSString).length
      let startOffset = min(max(range.startOffset, 0), textLength)
      let endOffset = min(max(range.endOffset, startOffset), textLength)

      return RuntimeHostTextInputRange(
        startOffset: startOffset,
        endOffset: endOffset
      )
    }

    /// Read one substring using the runtime text-range offsets.
    private func substring(
      _ text: String,
      range: RuntimeHostTextInputRange
    ) -> String {
      let nsString = text as NSString
      let length = max(range.endOffset - range.startOffset, 0)

      return nsString.substring(
        with: NSRange(
          location: range.startOffset,
          length: length
        )
      )
    }

    /// Build one selection range relative to one composing span.
    private func selectedRangeWithinComposing(
      selection: RuntimeHostTextInputRange?,
      composing: RuntimeHostTextInputRange
    ) -> NSRange {
      guard let selection else {
        return NSRange(location: 0, length: 0)
      }

      let location = min(
        max(selection.startOffset - composing.startOffset, 0),
        max(composing.endOffset - composing.startOffset, 0)
      )
      let endLocation = min(
        max(selection.endOffset - composing.startOffset, location),
        max(composing.endOffset - composing.startOffset, location)
      )

      return NSRange(
        location: location,
        length: endLocation - location
      )
    }
  }
#else
  /// The explicit non-UIKit text-input request surface used for host builds without UIKit.
  @MainActor
  public final class UIKitTextInputRequests: TextInputRequests {
    /// Create one unsupported UIKit text-input request surface.
    public init(
      events: any TextInputEvents
    ) {
      let _ = events
    }

    public func openTextInput(
      _ request: RuntimeHostTextInputOpenRequest
    ) -> UInt32 {
      let _ = request

      return hostStatusNotSupported
    }

    public func closeTextInput(
      _ request: RuntimeHostTextInputCloseRequest
    ) -> UInt32 {
      let _ = request

      return hostStatusNotSupported
    }

    public func setTextInputGeometry(
      _ request: RuntimeHostTextInputGeometryRequest
    ) -> UInt32 {
      let _ = request

      return hostStatusNotSupported
    }

    public func setTextInputState(
      _ request: RuntimeHostTextInputStateRequest
    ) -> UInt32 {
      let _ = request

      return hostStatusNotSupported
    }
  }
#endif
