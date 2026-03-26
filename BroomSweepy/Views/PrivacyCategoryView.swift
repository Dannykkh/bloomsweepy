import SwiftUI

struct PrivacyCategoryView: View {
    @Bindable var viewModel: CleanerViewModel

    var body: some View {
        PrivacyCleanerView(viewModel: viewModel)
    }
}
