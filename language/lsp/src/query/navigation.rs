use std::path::Path;

use tspp_lsp_server::jsonrpc;
use tspp_lsp_types as lsp;
use tspp_query as query;
use tspp_repository::Revision;

use super::{Document, DocumentSet, IntoLsp, QueryContinuation};
use crate::server::internal_error;

impl DocumentSet {
    /// Encode one navigation target as an LSP location link.
    pub(crate) fn location_link(
        &self,
        target: &query::NavigationTarget,
    ) -> jsonrpc::Result<lsp::LocationLink> {
        let origin = self.document(target.origin.span.file)?;
        let origin_selection_range = origin.range(target.origin.span)?;

        // build the destination
        let destination = self.document(target.target.span.file)?;
        let target_uri = destination.uri(self.project)?;
        let (target_range, target_selection_range) =
            destination.ranges(target.target.span, target.target.selection_span)?;

        Ok(lsp::LocationLink {
            origin_selection_range: Some(origin_selection_range),
            target_uri,
            target_range,
            target_selection_range,
        })
    }
}

#[allow(deprecated)]
impl DocumentSet {
    /// Encode one search symbol as LSP symbol information.
    pub(crate) fn symbol_information(
        &self,
        search_symbol: &query::SearchSymbol,
    ) -> jsonrpc::Result<lsp::SymbolInformation> {
        let document = self.document(search_symbol.target.span.file)?;
        let location = document.location(self.project, search_symbol.target.span)?;
        let kind = search_symbol.kind.into_lsp();

        Ok(lsp::SymbolInformation {
            name: search_symbol.name.clone(),
            kind,
            tags: None,
            deprecated: None,
            location,
            container_name: search_symbol.container.clone(),
        })
    }
}

impl DocumentSet {
    /// Encode one query call item as an LSP call hierarchy item.
    pub(crate) fn call_hierarchy_item(
        &self,
        path: &Path,
        revision: Revision,
        item: &query::CallItem,
    ) -> jsonrpc::Result<lsp::CallHierarchyItem> {
        let document = self.document(item.target.span.file)?;
        let uri = document.uri(self.project)?;
        let (range, selection_range) =
            document.ranges(item.target.span, item.target.selection_span)?;
        let kind = match item.kind {
            query::CallItemKind::Function => lsp::SymbolKind::FUNCTION,
            query::CallItemKind::Method => lsp::SymbolKind::METHOD,
            query::CallItemKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
        };

        let continuation = QueryContinuation::new(path, revision, item.clone());
        let data = Some(continuation.into_value()?);

        Ok(lsp::CallHierarchyItem {
            name: item.name.clone(),
            kind,
            tags: None,
            detail: item.signature.clone(),
            uri,
            range,
            selection_range,
            data,
        })
    }
}

impl DocumentSet {
    /// Encode one incoming call as an LSP incoming call.
    pub(crate) fn incoming_call(
        &self,
        path: &Path,
        revision: Revision,
        call: &query::IncomingCall,
    ) -> jsonrpc::Result<lsp::CallHierarchyIncomingCall> {
        let from = self.call_hierarchy_item(path, revision, &call.from)?;
        let document = self.document(call.from.target.span.file)?;
        if let Some(span) = call
            .from_ranges
            .iter()
            .find(|span| span.file != document.id())
        {
            return Err(internal_error(format!(
                "incoming call range belongs to another file: {span:?}"
            )));
        }

        let from_ranges = call
            .from_ranges
            .iter()
            .map(|span| document.range(*span))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(lsp::CallHierarchyIncomingCall { from, from_ranges })
    }
}

impl DocumentSet {
    /// Encode one outgoing call as an LSP outgoing call.
    pub(crate) fn outgoing_call(
        &self,
        path: &Path,
        revision: Revision,
        call: &query::OutgoingCall,
        source: &Document,
    ) -> jsonrpc::Result<lsp::CallHierarchyOutgoingCall> {
        let to = self.call_hierarchy_item(path, revision, &call.to)?;
        if let Some(span) = call
            .from_ranges
            .iter()
            .find(|span| span.file != source.id())
        {
            return Err(internal_error(format!(
                "outgoing call range belongs to another file: {span:?}"
            )));
        }

        let from_ranges = call
            .from_ranges
            .iter()
            .map(|span| source.range(*span))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Ok(lsp::CallHierarchyOutgoingCall { to, from_ranges })
    }
}

impl DocumentSet {
    /// Encode one query type item as an LSP type hierarchy item.
    pub(crate) fn type_hierarchy_item(
        &self,
        path: &Path,
        revision: Revision,
        item: &query::TypeItem,
    ) -> jsonrpc::Result<lsp::TypeHierarchyItem> {
        let document = self.document(item.target.span.file)?;
        let uri = document.uri(self.project)?;
        let (range, selection_range) =
            document.ranges(item.target.span, item.target.selection_span)?;
        let kind = item.kind.into_lsp();

        let continuation = QueryContinuation::new(path, revision, item.clone());
        let data = Some(continuation.into_value()?);

        Ok(lsp::TypeHierarchyItem {
            name: item.name.clone(),
            kind,
            tags: None,
            detail: item.generics.clone(),
            uri,
            range,
            selection_range,
            data,
        })
    }
}
