import SwiftUI

// The installed SDK exports a new @State macro but Command Line Tools does not
// include its plugin. This alias selects the established property-wrapper type.
typealias ViewState<Value> = SwiftUI.State<Value>
