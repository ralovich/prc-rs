// -*- mode: c++; tab-width: 2; indent-tabs-mode: nil; c-basic-offset: 2; coding: utf-8-unix -*-

#include <cstdio>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>
#include <prc.h>

std::vector<char> undump(const char* file_name)
{
    std::ifstream file(file_name, std::ios::binary);
    return std::vector<char>((std::istreambuf_iterator<char>(file)),
                              std::istreambuf_iterator<char>());
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

  auto bytes = undump(args.at(1).c_str());
  //std::cout << bytes.size() << std::endl;
  std::vector<char> json(1024*1024*32);
  uint64_t json_returned_bytes = 0;
  int rv = prc_parse_to_json(bytes.size(),
                             bytes.data(),
                             json.size(),
                             &json[0],
                             &json_returned_bytes);
  //std::cout << json_returned_bytes << std::endl;
  //printf("%d %s\n", rv, ((rv==0)?"prc-rs returned SUCCESS":"FAIL"));
  if (rv !=  0) {
    return -2;
  }

  json.resize(json_returned_bytes);
  std::cout << std::string(json.data()) << std::endl;

  return 0;
}
