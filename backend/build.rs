fn main() {
	build_info_build::build_script().collect_dependencies(build_info_build::DependencyDepth::Depth(1));
}
