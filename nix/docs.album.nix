# this file is the documentation of all album.nix attributes

{ munix }:

munix.mkAlbum {

  # album name in "{albumartist}-{album} -> lowercase -> sanitize with `-`" format
  name = "";

  origin = {
    # auto resolved by rust based on selected source
    path = "";
    # NAR hash of origin.path
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  fetch = {
    torrent = {
      file = ./Info/source.torrent;
      # file hash of fetch.torrent.file
      hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    };
    web = {
      url = "";
      # NAR hash of files fetched by source.web.url 
      hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    };
  };

  cover = {
    file = ./cover.png;
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  album = {
    metadata = {
    };
    mbid = {
    };
  };

  tracks = [
    # repeat for every file found by --tracks expression
    {
      file = "";
      metadata = {
      };
      mbid = {
      };
    }
  ];
}
