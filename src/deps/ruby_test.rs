use super::*;

fn chains(file: &RubyFile) -> Vec<String> {
    file.refs.iter().map(|r| r.chain.join("::")).collect()
}

fn reference(chain: &[&str], scopes: &[&str]) -> Reference {
    Reference {
        chain: chain.iter().map(|s| s.to_string()).collect(),
        scopes: scopes.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn requires_of_every_form() {
    let src = "require 'shop/cart'\nrequire_relative \"../helpers\"\nrequire(\"json\")\nload 'tasks.rb'\nautoload :Cart, 'shop/cart'\nrequire File.expand_path('../support/a', __FILE__)\nrequire File.join(__dir__, 'b')\nrequire \"#{__dir__}/c\"\nrequire File.join(File.dirname(__FILE__), 'd')\nrequire \"plugins/#{name}\"\n";
    let file = read(src);
    assert_eq!(
        file.requires,
        vec![
            Require::LoadPath("shop/cart".into()),
            Require::Relative("../helpers".into()),
            Require::LoadPath("json".into()),
            Require::LoadPath("tasks.rb".into()),
            Require::LoadPath("shop/cart".into()),
            Require::Relative("support/a".into()),
            Require::Relative("b".into()),
            Require::Relative("c".into()),
            Require::Relative("d".into()),
        ]
    );
}

#[test]
fn nested_definitions_are_qualified() {
    let src = "module Shop\n  class Cart < Base\n    LIMIT = 10\n    def total\n      Item.sum\n    end\n  end\n\n  class Error < StandardError; end\nend\n";
    let file = read(src);
    assert_eq!(
        file.defines,
        vec!["Shop::Cart::LIMIT", "Shop::Cart", "Shop::Error"]
    );
    assert!(file.refs.contains(&reference(&["Base"], &["Shop", ""])));
    assert!(
        file.refs
            .contains(&reference(&["Item"], &["Shop::Cart", "Shop", ""]))
    );
    assert!(
        file.refs
            .contains(&reference(&["StandardError"], &["Shop", ""]))
    );
}

#[test]
fn a_module_that_only_holds_definitions_is_a_namespace() {
    let file = read(
        "module Shop\n  module Admin\n    class Report\n      def x; end\n    end\n  end\nend\n",
    );
    assert_eq!(file.defines, vec!["Shop::Admin::Report"]);
}

#[test]
fn an_empty_class_is_a_definition() {
    let file = read(
        "class Log < ActiveRecord::Base\nend\n\nmodule Marker\nend\n\nclass Shop < Base\n  class Cart\n  end\nend\n",
    );
    assert_eq!(file.defines, vec!["Log", "Marker", "Shop::Cart", "Shop"]);
}

#[test]
fn constants_assigned_in_a_namespace_define_it() {
    let file = read("module Shop\n  VERSION = \"1.0\"\n  Point = Struct.new(:x)\n  X == 1\nend\n");
    assert_eq!(file.defines, vec!["Shop::VERSION", "Shop::Point", "Shop"]);
}

#[test]
fn compact_names_look_up_from_themselves_and_the_top() {
    let file = read(
        "class Admin::UsersController < ApplicationController\n  def index\n    User.all\n  end\nend\n",
    );
    assert_eq!(file.defines, vec!["Admin::UsersController"]);
    assert!(
        file.refs
            .contains(&reference(&["User"], &["Admin::UsersController", ""]))
    );
}

#[test]
fn qualified_absolute_and_method_references() {
    let file = read(
        "x = ::Shop::Cart.new\ny = Shop::Cart::LIMIT\nz = obj.Foo\nShop::Cart.find\nw = Shop::build\n",
    );
    assert_eq!(
        chains(&file),
        vec!["Shop", "Shop::Cart", "Shop::Cart::LIMIT"]
    );
    assert!(file.refs.contains(&reference(&["Shop", "Cart"], &[""])));
}

#[test]
fn singleton_class_stays_in_its_class() {
    let file =
        read("class Cart\n  class << self\n    def build\n      Item.new\n    end\n  end\nend\n");
    assert_eq!(file.defines, vec!["Cart"]);
    assert!(file.refs.contains(&reference(&["Item"], &["Cart", ""])));
}

#[test]
fn comments_strings_and_heredocs_name_nothing() {
    let src = "# Uses Foo\nx = \"Bar\"\ny = <<~TXT\n  Baz::Qux\nTXT\nz = 'Quux'\n";
    assert!(read(src).refs.is_empty());
}
