// -*- mode: c++; tab-width: 2; indent-tabs-mode: nil; c-basic-offset: 2; coding: utf-8-unix -*-

//#include <cstdio>
#include <cstring>
#include <fstream>
#include <iostream>
#include <memory>
#include <string>
#include <vector>
#include <prc.h>

std::vector<char> undump(const char* file_name)
{
    std::ifstream file(file_name, std::ios::binary);
    return std::vector<char>((std::istreambuf_iterator<char>(file)),
                              std::istreambuf_iterator<char>());
}

void* allocate(const size_t num_bytes)
{
    return new unsigned char[num_bytes];
}

int main(int argc, char** argv)
{
  std::vector<std::string> args(argv, argv+argc);
  for(auto& arg : args) {
    //std::cout << arg << std::endl;
  }
  if (args.size() < 2) {
    return -1;
  }

  auto in_file = args.at(1);

  auto bytes = undump(in_file.c_str());
  //std::cout << bytes.size() << std::endl;
  uint64_t json_returned_bytes = 0;

  unsigned char* dst;
  int rv = prc_parse_to_json(bytes.size(),
                             reinterpret_cast<unsigned char*>(bytes.data()),
                             allocate,
                             &dst,
                             &json_returned_bytes);
  //std::cout << json_returned_bytes << std::endl;
  //printf("%d %s\n", rv, ((rv==0)?"prc-rs returned SUCCESS":"FAIL"));
  if (rv != 0) {
      return -2;
  }
  if (!dst) {
      return -3;
  }

  std::unique_ptr<unsigned char[]> dst_owner(dst);
  std::string json_str(reinterpret_cast<char*>(dst_owner.get()), json_returned_bytes);

  if (args.size() > 2) {
    auto out_file = args.at(2);
    std::ofstream ofs (out_file, std::ofstream::out);
    ofs << json_str << std::endl;
  }
  else {
    std::cout << json_str << std::endl;
  }

  return 0;
}
