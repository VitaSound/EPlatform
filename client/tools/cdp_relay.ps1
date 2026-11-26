# TCP relay: 0.0.0.0:9223 -> 127.0.0.1:9222 (no admin needed)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @"
using System;
using System.Net;
using System.Net.Sockets;
using System.Threading.Tasks;
public static class CdpRelay {
  public static void Run(int listenPort, string targetHost, int targetPort) {
    var listener = new TcpListener(IPAddress.Any, listenPort);
    listener.Start();
    Console.WriteLine("relay 0.0.0.0:" + listenPort + " -> " + targetHost + ":" + targetPort);
    while (true) {
      var client = listener.AcceptTcpClient();
      Console.WriteLine("client " + client.Client.RemoteEndPoint);
      Task.Run(() => Handle(client, targetHost, targetPort));
    }
  }
  static async Task Handle(TcpClient client, string host, int port) {
    try {
      using (client)
      using (var upstream = new TcpClient()) {
        await upstream.ConnectAsync(host, port).ConfigureAwait(false);
        var a = client.GetStream();
        var b = upstream.GetStream();
        var t1 = a.CopyToAsync(b);
        var t2 = b.CopyToAsync(a);
        await Task.WhenAny(t1, t2).ConfigureAwait(false);
      }
    } catch (Exception ex) {
      Console.WriteLine(ex.Message);
    }
  }
}
"@
[CdpRelay]::Run(9223, '127.0.0.1', 9222)
