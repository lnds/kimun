use super::*;
use crate::deps::graph::FileGraph;
use crate::deps::layout::Layout;
use std::path::{Path, PathBuf};

fn ruby(path: &str, text: &str) -> Source {
    Source {
        path: PathBuf::from(path),
        language: "Ruby".to_string(),
        text: text.to_string(),
        templates: Vec::new(),
    }
}

/// The paths `from` uses in the graph of `sources`, sorted.
fn used(sources: &[Source], from: &str) -> Vec<String> {
    let graph = FileGraph::build(sources, &Layout::default(), &|_, _| false);
    let mut used: Vec<String> = graph.edges()[Path::new(from)]
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    used.sort();
    used
}

#[test]
fn underscore_as_zeitwerk() {
    assert_eq!(underscore("Shop::LineItem"), "shop/line_item");
    assert_eq!(underscore("HTTPClient"), "http_client");
    assert_eq!(underscore("Api2Token"), "api2_token");
}

#[test]
fn require_relative_resolves_from_the_file() {
    let sources = [
        ruby(
            "lib/shop.rb",
            "require_relative 'shop/cart'\nrequire_relative './missing'\n",
        ),
        ruby("lib/shop/cart.rb", "require_relative '../util.rb'\n"),
        ruby("lib/util.rb", ""),
    ];
    assert_eq!(used(&sources, "lib/shop.rb"), vec!["lib/shop/cart.rb"]);
    assert_eq!(used(&sources, "lib/shop/cart.rb"), vec!["lib/util.rb"]);
}

#[test]
fn require_looks_in_the_load_path_and_drops_gems() {
    let sources = [
        ruby(
            "test/cart_test.rb",
            "require 'test_helper'\nrequire 'shop/cart'\nrequire 'json'\n",
        ),
        ruby("test/test_helper.rb", ""),
        ruby("lib/shop/cart.rb", ""),
        ruby("contrib/lib/shop/cart.rb", ""),
    ];
    assert_eq!(
        used(&sources, "test/cart_test.rb"),
        vec!["lib/shop/cart.rb", "test/test_helper.rb"]
    );
}

#[test]
fn a_gem_of_the_repository_requires_another() {
    let sources = [
        ruby(
            "sinatra-contrib/lib/sinatra/cookies.rb",
            "require 'sinatra/base'\n",
        ),
        ruby(
            "lib/sinatra/base.rb",
            "module Sinatra\n  class Base\n  end\nend\n",
        ),
    ];
    assert_eq!(
        used(&sources, "sinatra-contrib/lib/sinatra/cookies.rb"),
        vec!["lib/sinatra/base.rb"]
    );
}

#[test]
fn rails_constants_resolve_without_requires() {
    let sources = [
        ruby(
            "app/controllers/admin/users_controller.rb",
            "module Admin\n  class UsersController < ApplicationController\n    def index\n      @users = User.active\n      Reports::Weekly.call\n    end\n  end\nend\n",
        ),
        ruby(
            "app/controllers/application_controller.rb",
            "class ApplicationController < ActionController::Base\nend\n",
        ),
        ruby(
            "app/models/user.rb",
            "class User < ApplicationRecord\n  include Trackable\nend\n",
        ),
        ruby(
            "app/models/concerns/trackable.rb",
            "module Trackable\n  extend ActiveSupport::Concern\nend\n",
        ),
        ruby(
            "app/services/reports/weekly.rb",
            "module Reports\n  class Weekly\n  end\nend\n",
        ),
    ];
    assert_eq!(
        used(&sources, "app/controllers/admin/users_controller.rb"),
        vec![
            "app/controllers/application_controller.rb",
            "app/models/user.rb",
            "app/services/reports/weekly.rb",
        ]
    );
    assert_eq!(
        used(&sources, "app/models/user.rb"),
        vec!["app/models/concerns/trackable.rb"]
    );
}

#[test]
fn a_namespace_resolves_from_the_inside_out() {
    let sources = [
        ruby(
            "lib/shop/cart.rb",
            "module Shop\n  class Cart\n    def x\n      Item.new\n      Shop::Item\n      Config.load\n    end\n  end\nend\n",
        ),
        ruby(
            "lib/shop/item.rb",
            "module Shop\n  class Item\n  end\nend\n",
        ),
        ruby("lib/item.rb", "class Item\nend\n"),
        ruby("lib/config.rb", "class Config\nend\n"),
    ];
    assert_eq!(
        used(&sources, "lib/shop/cart.rb"),
        vec!["lib/config.rb", "lib/shop/item.rb"]
    );
}

#[test]
fn a_constant_is_found_whole() {
    let sources = [
        ruby(
            "lib/a.rb",
            "x = Shop::Cart::LIMIT
y = Rack::Handler::WEBrick
",
        ),
        ruby(
            "lib/shop/cart.rb",
            "module Shop
  class Cart
    LIMIT = 3
  end
end
",
        ),
        ruby(
            "test/server_test.rb",
            "module Rack::Handler
  class Mock
  end
  register :mock, Mock
end
",
        ),
    ];
    assert_eq!(used(&sources, "lib/a.rb"), vec!["lib/shop/cart.rb"]);
}

#[test]
fn a_reopened_class_resolves_to_the_file_named_after_it() {
    let sources = [
        ruby(
            "app/models/order.rb",
            "class Order\n  def total; end\nend\n",
        ),
        ruby(
            "lib/extensions/order_ext.rb",
            "class Order\n  def legacy; end\nend\n",
        ),
        ruby(
            "app/jobs/bill.rb",
            "class Bill\n  def x\n    Order.find\n  end\nend\n",
        ),
    ];
    assert_eq!(
        used(&sources, "app/jobs/bill.rb"),
        vec!["app/models/order.rb"]
    );
}

#[test]
fn without_a_file_named_after_it_every_nearest_definition_counts() {
    let sources = [
        ruby("lib/shop/parts_a.rb", "class Cart\n  def a; end\nend\n"),
        ruby("lib/shop/parts_b.rb", "class Cart\n  def b; end\nend\n"),
        ruby("other/cart_mirror.rb", "class Cart\n  def c; end\nend\n"),
        ruby("lib/shop/use.rb", "Cart.new\n"),
    ];
    assert_eq!(
        used(&sources, "lib/shop/use.rb"),
        vec!["lib/shop/parts_a.rb", "lib/shop/parts_b.rb"]
    );
}

#[test]
fn what_tests_define_is_seen_from_tests_only() {
    let sources = [
        ruby("lib/server.rb", "Rackup::Handler.pick(name)\n"),
        ruby(
            "test/server_test.rb",
            "module Rackup::Handler\n  register :mock, Mock\nend\n",
        ),
        ruby(
            "spec/support/helpers.rb",
            "module Helpers\n  def sign_in; end\nend\n",
        ),
        ruby("spec/a_spec.rb", "include Helpers\nRackup::Handler\n"),
    ];
    assert!(used(&sources, "lib/server.rb").is_empty());
    assert_eq!(
        used(&sources, "spec/a_spec.rb"),
        vec!["spec/support/helpers.rb", "test/server_test.rb"]
    );
}

#[test]
fn reopenings_spread_over_directories_are_ambiguous() {
    let sources = [
        ruby(
            "contrib/lib/shop/json.rb",
            "module Shop
  register Json
end
",
        ),
        ruby(
            "contrib/lib/shop/xml.rb",
            "module Shop
  register Xml
end
",
        ),
        ruby(
            "lib/shop/base.rb",
            "module Shop
  def self.new; end
end
",
        ),
        ruby(
            "test/a_test.rb",
            "Shop.new
",
        ),
    ];
    let graph = FileGraph::build(&sources, &Layout::default(), &|_, _| false);
    assert!(graph.edges()[Path::new("test/a_test.rb")].is_empty());
    assert_eq!(graph.ambiguous, 1);
}

#[test]
fn reopened_core_classes_are_no_use() {
    let sources = [
        ruby(
            "lib/core_ext/string.rb",
            "class String\n  def shout; upcase; end\nend\n",
        ),
        ruby("lib/a.rb", "x = String.new\ny = Hash.new\n"),
    ];
    assert!(used(&sources, "lib/a.rb").is_empty());
}

#[test]
fn a_namespace_only_module_is_no_definition() {
    let sources = [
        ruby(
            "lib/shop/cart.rb",
            "module Shop\n  class Cart\n  end\nend\n",
        ),
        ruby("lib/a.rb", "Shop.configure\n"),
    ];
    assert!(used(&sources, "lib/a.rb").is_empty());
}

#[test]
fn constants_no_file_defines_are_found_by_their_path() {
    let sources = [
        ruby(
            "app/models/legacy_record.rb",
            "LegacyRecord = Class.new(ApplicationRecord) do\nend\n",
        ),
        ruby(
            "app/models/dynamic.rb",
            "Object.const_set(:Dynamic, Class.new)\n",
        ),
        ruby(
            "app/jobs/sync.rb",
            "class Sync\n  def x\n    Dynamic.new\n    LegacyRecord.all\n  end\nend\n",
        ),
    ];
    assert_eq!(
        used(&sources, "app/jobs/sync.rb"),
        vec!["app/models/dynamic.rb", "app/models/legacy_record.rb"]
    );
}

#[test]
fn ruby_is_measured() {
    assert!(crate::deps::is_supported("Ruby"));
}
